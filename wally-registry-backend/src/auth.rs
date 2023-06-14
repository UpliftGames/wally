use std::fmt;

use anyhow::{format_err, Context};
use constant_time_eq::constant_time_eq;
use libwally::{package_id::PackageId, package_index::PackageIndex};
use reqwest::Client;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::{config::Config, error::ApiErrorStatus};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum AuthMode {
    ApiKey(String),
    DoubleApiKey { read: Option<String>, write: String },
    GithubOAuth,
    Unauthenticated,
}

#[derive(Deserialize)]
pub struct GithubOrgInfo {
    login: String,
}

#[derive(Deserialize)]
pub struct GithubOrgInfoResponse {
    organization: GithubOrgInfo,
}

#[derive(Deserialize)]
pub struct GithubUserInfo {
    login: String,
    id: u64,
}

#[derive(Deserialize)]
pub struct GithubInfo {
    user: GithubUserInfo,
    orgs: Vec<String>,
}

impl GithubInfo {
    pub fn login(&self) -> &str {
        &self.user.login
    }

    pub fn id(&self) -> &u64 {
        &self.user.id
    }
}

impl fmt::Debug for AuthMode {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthMode::ApiKey(_) => write!(formatter, "API key"),
            AuthMode::DoubleApiKey { .. } => write!(formatter, "double API key"),
            AuthMode::GithubOAuth => write!(formatter, "Github OAuth"),
            AuthMode::Unauthenticated => write!(formatter, "no authentication"),
        }
    }
}

fn match_api_key<T>(request: &Request<'_>, key: &str, result: T) -> Outcome<T, Error> {
    let input_api_key: String = match request.headers().get_one("authorization") {
        Some(key) if key.starts_with("Bearer ") => (key[6..].trim()).to_owned(),
        _ => {
            return format_err!("API key required")
                .status(Status::Unauthorized)
                .into();
        }
    };

    if constant_time_eq(key.as_bytes(), input_api_key.as_bytes()) {
        Outcome::Success(result)
    } else {
        format_err!("Invalid API key for read access")
            .status(Status::Unauthorized)
            .into()
    }
}

async fn verify_github_token(request: &Request<'_>) -> Outcome<WriteAccess, Error> {
    let token: String = match request.headers().get_one("authorization") {
        Some(key) if key.starts_with("Bearer ") => (key[6..].trim()).to_owned(),
        _ => {
            return format_err!("Github auth required")
                .status(Status::Unauthorized)
                .into();
        }
    };

    let orgs = get_github_orgs(&token).await.unwrap_or_else(|err| {
        eprintln!("{:?}", err);
        Vec::new()
    });

    let user_info = get_github_user_info(&token)
        .await
        .map(|user| GithubInfo { user, orgs });

    match user_info {
        Err(err) => format_err!("Github auth failed: {}", err)
            .status(Status::Unauthorized)
            .into(),
        Ok(info) => Outcome::Success(WriteAccess::Github(info)),
    }
}

async fn get_github_user_info(token: &str) -> anyhow::Result<GithubUserInfo> {
    let client = Client::new();

    // Users already logged in may not have given us read:org permission so we
    // need to still support a basic read:user check.
    // See: https://github.com/UpliftGames/wally/pull/147
    // TODO: Eventually we can transition to only using org level oauth
    let response = client
        .get("https://api.github.com/user")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(token)
        .send()
        .await
        .context("Github user info request failed!")?;

    response
        .json::<GithubUserInfo>()
        .await
        .context("Failed to parse github user info")
}

pub async fn get_github_orgs(token: &str) -> Result<Vec<String>, Error> {
    let client = Client::new();

    let org_response = client
        .get("https://api.github.com/user/memberships/orgs")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(token)
        .send()
        .await
        .context("Github org membership request failed")?;

    let github_org_info = org_response
        .json::<Vec<GithubOrgInfoResponse>>()
        .await
        .context("Failed to parse github org membership")?;

    let orgs: Vec<_> = github_org_info
        .iter()
        .map(|org_info| org_info.organization.login.to_lowercase())
        .collect();

    Ok(orgs)
}

pub enum ReadAccess {
    Public,
    ApiKey,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ReadAccess {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Error> {
        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("AuthMode was not configured");

        match &config.auth {
            AuthMode::Unauthenticated => Outcome::Success(ReadAccess::Public),
            AuthMode::GithubOAuth => Outcome::Success(ReadAccess::Public),
            AuthMode::ApiKey(key) => match_api_key(request, key, ReadAccess::ApiKey),
            AuthMode::DoubleApiKey { read, .. } => match read {
                None => Outcome::Success(ReadAccess::Public),
                Some(key) => match_api_key(request, key, ReadAccess::ApiKey),
            },
        }
    }
}

pub enum WriteAccess {
    ApiKey,
    Github(GithubInfo),
}

pub enum WritePermission {
    Default,
    Owner,
    User,
    Org,
}

impl WriteAccess {
    pub async fn can_write_package(
        &self,
        package_id: &PackageId,
        index: &PackageIndex,
    ) -> anyhow::Result<Option<WritePermission>> {
        let scope = package_id.name().scope();

        match self {
            WriteAccess::ApiKey => Ok(Some(WritePermission::Default)),
            WriteAccess::Github(info) => github_write_permission_for_scope(info, scope, index),
        }
    }
}

fn github_write_permission_for_scope(
    info: &GithubInfo,
    scope: &str,
    index: &PackageIndex,
) -> anyhow::Result<Option<WritePermission>> {
    Ok(match index.is_scope_owner(scope, info.id())? {
        true => Some(WritePermission::Owner),
        false => {
            // Only grant write access if the username matches the scope AND the scope has no existing owners
            if info.login().to_lowercase() == scope && index.get_scope_owners(scope)?.is_empty() {
                Some(WritePermission::User)
            // ... or if they are in the organization!
            } else if info.orgs.contains(&scope.to_string()) {
                Some(WritePermission::Org)
            } else {
                None
            }
        }
    })
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WriteAccess {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Error> {
        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("AuthMode was not configured");

        match &config.auth {
            AuthMode::Unauthenticated => format_err!("Invalid API key for write access")
                .status(Status::Unauthorized)
                .into(),
            AuthMode::ApiKey(key) => match_api_key(request, key, WriteAccess::ApiKey),
            AuthMode::DoubleApiKey { write, .. } => {
                match_api_key(request, write, WriteAccess::ApiKey)
            }
            AuthMode::GithubOAuth => verify_github_token(request).await,
        }
    }
}
