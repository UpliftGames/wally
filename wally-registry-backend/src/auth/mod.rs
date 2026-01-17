use std::{collections::HashMap, fmt};

use anyhow::{anyhow, format_err};
use constant_time_eq::constant_time_eq;
use libwally::{package_id::PackageId, package_index::PackageIndex};
use reqwest::{Client, StatusCode};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::{config::Config, error::ApiErrorStatus};

#[cfg(test)]
mod tests;

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum AuthMode {
    ApiKey(String),
    DoubleApiKey {
        read: Option<String>,
        write: String,
    },
    GithubOAuth {
        #[serde(rename = "client-id")]
        client_id: String,
        #[serde(rename = "client-secret")]
        client_secret: String,
    },
    Unauthenticated,
}

#[derive(Deserialize)]
pub struct GithubInfo {
    login: String,
    id: u64,
    #[serde(skip)]
    orgs: Vec<String>,
}

#[derive(Deserialize)]
struct GithubOrg {
    login: String,
}

#[derive(Deserialize)]
struct GithubOrgMembership {
    role: String,
}

#[derive(Deserialize)]
struct GithubOrgSettings {
    members_can_create_repositories: Option<bool>,
}

impl GithubInfo {
    pub fn login(&self) -> &str {
        &self.login
    }

    pub fn id(&self) -> &u64 {
        &self.id
    }

    pub fn orgs(&self) -> &[String] {
        &self.orgs
    }
}

#[derive(Deserialize)]
#[allow(unused)] // Variables are (currently) not accessed but ensure they are present during json parsing
struct ValidatedGithubApp {
    client_id: String,
}

#[derive(Deserialize)]
#[allow(unused)] // Variables are (currently) not accessed but ensure they are present during json parsing
struct ValidatedGithubInfo {
    id: u64,
    app: ValidatedGithubApp,
}

impl fmt::Debug for AuthMode {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthMode::ApiKey(_) => write!(formatter, "API key"),
            AuthMode::DoubleApiKey { .. } => write!(formatter, "double API key"),
            AuthMode::GithubOAuth { .. } => write!(formatter, "Github OAuth"),
            AuthMode::Unauthenticated => write!(formatter, "no authentication"),
        }
    }
}

async fn can_user_create_in_org(client: &Client, token: &str, org: &str) -> bool {
    let membership_url = format!("https://api.github.com/user/memberships/orgs/{}", org);
    if let Ok(response) = client
        .get(&membership_url)
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(token)
        .send()
        .await
    {
        if let Ok(membership) = response.json::<GithubOrgMembership>().await {
            if membership.role == "admin" {
                return true;
            }
        }
    }

    let org_url = format!("https://api.github.com/orgs/{}", org);
    if let Ok(response) = client
        .get(&org_url)
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(token)
        .send()
        .await
    {
        if let Ok(settings) = response.json::<GithubOrgSettings>().await {
            return settings.members_can_create_repositories.unwrap_or(false);
        }
    }

    false
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

async fn verify_github_token(
    request: &Request<'_>,
    client_id: &str,
    client_secret: &str,
) -> Outcome<WriteAccess, Error> {
    let token: String = match request.headers().get_one("authorization") {
        Some(key) if key.starts_with("Bearer ") => (key[6..].trim()).to_owned(),
        _ => {
            return format_err!("Github auth required")
                .status(Status::Unauthorized)
                .into();
        }
    };

    let client = Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(&token)
        .send()
        .await;

    let mut github_info = match response {
        Err(err) => {
            return format_err!(err).status(Status::InternalServerError).into();
        }
        Ok(response) => match response.json::<GithubInfo>().await {
            Err(err) => {
                return format_err!("Github auth failed: {}", err)
                    .status(Status::Unauthorized)
                    .into();
            }
            Ok(github_info) => github_info,
        },
    };

    // Fetch user's org memberships and filter to orgs where user can create repos
    let orgs_response = client
        .get("https://api.github.com/user/orgs")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(&token)
        .send()
        .await;

    if let Ok(response) = orgs_response {
        if let Ok(orgs) = response.json::<Vec<GithubOrg>>().await {
            for org in orgs {
                if can_user_create_in_org(&client, &token, &org.login).await {
                    github_info.orgs.push(org.login.to_lowercase());
                }
            }
        }
    }

    let mut body = HashMap::new();
    body.insert("access_token", &token);

    let response = client
        .post(format!(
            "https://api.github.com/applications/{}/token",
            client_id
        ))
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .basic_auth(client_id, Some(client_secret))
        .json(&body)
        .send()
        .await;

    match response {
        Err(err) => format_err!(err).status(Status::InternalServerError).into(),
        Ok(response) => {
            // https://docs.github.com/en/rest/apps/oauth-applications#check-a-token--status-codes
            match response.status() {
                StatusCode::OK => {
                    // Token was issued by our OAuth app - validate the response
                    match response.json::<ValidatedGithubInfo>().await {
                        Ok(_) => Outcome::Success(WriteAccess::Github(github_info)),
                        Err(err) => format_err!("Github auth failed: {}", err)
                            .status(Status::Unauthorized)
                            .into(),
                    }
                }
                StatusCode::NOT_FOUND => {
                    // Token is valid for GitHub (we successfully called /user above)
                    // but wasn't issued by our OAuth app. This happens with:
                    // - Personal Access Tokens (PATs)
                    // - Fine-grained PATs
                    // - GitHub Actions tokens
                    // We trust the github_info from /user since that call succeeded.
                    Outcome::Success(WriteAccess::Github(github_info))
                }
                StatusCode::UNPROCESSABLE_ENTITY => anyhow!("GitHub auth was invalid")
                    .status(Status::Unauthorized)
                    .into(),
                status => format_err!("Github auth failed because: {}", status)
                    .status(Status::UnprocessableEntity)
                    .into(),
            }
        }
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
            AuthMode::GithubOAuth { .. } => Outcome::Success(ReadAccess::Public),
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

impl WriteAccess {
    pub fn can_write_package(
        &self,
        package_id: &PackageId,
        index: &PackageIndex,
    ) -> anyhow::Result<bool> {
        let scope = package_id.name().scope();

        let has_permission = match self {
            WriteAccess::ApiKey => true,
            WriteAccess::Github(github_info) => {
                if index.is_scope_owner(scope, github_info.id())? {
                    return Ok(true);
                }

                let scope_has_no_owners = index.get_scope_owners(scope)?.is_empty();
                if !scope_has_no_owners {
                    return Ok(false);
                }

                let username_matches = github_info.login().to_lowercase() == scope;
                let user_is_org_member = github_info.orgs().contains(&scope.to_owned());

                username_matches || user_is_org_member
            }
        };

        Ok(has_permission)
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
            AuthMode::GithubOAuth {
                client_id,
                client_secret,
            } => verify_github_token(request, client_id, client_secret).await,
        }
    }
}
