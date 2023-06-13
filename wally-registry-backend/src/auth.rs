use std::fmt;

use anyhow::format_err;
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

#[derive(Deserialize, Clone, Debug)]
pub struct GithubInfo {
    login: String,
    id: u64,
}

#[derive(Deserialize, Debug)]
pub struct GithubOrgInfoOrganization {
    login: String,
}

#[derive(Deserialize, Debug)]
pub struct GithubOrgInfo {
    organization: GithubOrgInfoOrganization
}

#[derive(Debug)]
pub struct GithubWriteAccessInfo {
    pub user: GithubInfo,
    pub token: String,
}

impl GithubInfo {
    pub fn login(&self) -> &str {
        &self.login
    }

    pub fn id(&self) -> &u64 {
        &self.id
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

    let client = Client::new();

    // The user is in no orgs we can see so we cannot get their userinfo from that.
    let response = client
        .get("https://api.github.com/user")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(&token)
        .send()
        .await;

    let github_info = match response {
        Err(err) => {
            return format_err!(err).status(Status::InternalServerError).into();
        }
        Ok(response) => response.json::<GithubInfo>().await,
    };

    match github_info {
        Err(err) => format_err!("Github auth failed: {}", err)
            .status(Status::Unauthorized)
            .into(),
        Ok(github_info) => {
            return Outcome::Success(WriteAccess::Github(GithubWriteAccessInfo {
                user: github_info,
                token: token
            }));
        }
    }
}

pub async fn get_github_orgs(token: String) ->  Result<Vec<String>, Error> {
    let client = Client::new();

    let org_response = client
        .get("https://api.github.com/user/memberships/orgs")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(&token)
        .send()
        .await;

    let github_org_info = match org_response {
        Err(err) => {
            return Err(format_err!(err).status(Status::InternalServerError));
        }
        Ok(response) => response.json::<Vec<GithubOrgInfo>>().await,
    };

    match github_org_info {
        Ok(github_org_info) => {
            match github_org_info.get(0) {
                Some(_) => Ok(github_org_info
                .iter()
                .map(|x| x.organization.login.to_lowercase())
                .collect::<Vec<_>>()),
                None => Ok(vec![])
            }
        }
        Err(err) => Err(format_err!("Github auth failed: {}", err)
            .status(Status::Unauthorized)),
    }
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
    Github(GithubWriteAccessInfo),
}

pub enum WritePermission {
    Default,
    Org,
}

impl WriteAccess {
    pub async fn can_write_package(
        &self,
        package_id: &PackageId,
        index: &PackageIndex,
    ) -> Result<Option<WritePermission>, Error> {
        let scope = package_id.name().scope();

        let write_permission = match self {
            WriteAccess::ApiKey => Some(WritePermission::Default),
            WriteAccess::Github(github_info) => {
                match index.is_scope_owner(scope, github_info.user.id())? {
                    true => Some(WritePermission::Default),
                    // Only grant write access if the username matches the scope AND the scope has no existing owners or they are a member of the org
                    false => {
                        if github_info.user.login().to_lowercase() == scope
                            && index.get_scope_owners(scope)?.is_empty()
                        {
                            Some(WritePermission::Default)
                        } else {
                            let orgs = get_github_orgs(github_info.token.clone()).await;
                            match orgs {
                                Ok(orgs) => {
                                    if orgs.contains(&scope.to_string()) {
                                        Some(WritePermission::Org)
                                    } else {
                                        None
                                    }
                                },
                                Err(err) => {
                                    return Err(format_err!("Failed to get Github Organisations, do you need to re-login. Error: {:?}", err)
                                    .status(Status::Unauthorized)
                                    .into())
                                },
                            }
                            
                        }
                    }
                }
            }
        };

        Ok(write_permission)
    }
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
