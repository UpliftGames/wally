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

use crate::config::Config;

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum AuthMode {
    ApiKey(String),
    DoubleApiKey { read: Option<String>, write: String },
    GithubOAuth,
    Unauthenticated,
}

#[derive(Deserialize)]
pub struct GithubInfo {
    login: String,
    id: u64,
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

fn match_api_key<T>(request: &Request<'_>, key: &str, result: T) -> Outcome<T, anyhow::Error> {
    let input_api_key: String = match request.headers().get_one("authorization") {
        Some(key) if key.starts_with("Bearer ") => (key[6..].trim()).to_owned(),
        _ => {
            return Outcome::Failure((Status::Unauthorized, format_err!("API key required")));
        }
    };

    if constant_time_eq(key.as_bytes(), input_api_key.as_bytes()) {
        Outcome::Success(result)
    } else {
        Outcome::Failure((
            Status::Unauthorized,
            format_err!("Invalid API key for read access"),
        ))
    }
}

async fn verify_github_token(request: &Request<'_>) -> Outcome<WriteAccess, anyhow::Error> {
    let token: String = match request.headers().get_one("authorization") {
        Some(key) if key.starts_with("Bearer ") => (key[6..].trim()).to_owned(),
        _ => {
            return Outcome::Failure((Status::Unauthorized, format_err!("Github auth required")));
        }
    };

    let client = Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(token)
        .send()
        .await;

    let github_info = match response {
        Err(err) => {
            return Outcome::Failure((Status::InternalServerError, format_err!(err)));
        }
        Ok(response) => response.json::<GithubInfo>().await,
    };

    match github_info {
        Err(err) => Outcome::Failure((
            Status::Unauthorized,
            format_err!("Github auth failed: {}", err),
        )),
        Ok(github_info) => Outcome::Success(WriteAccess::Github(github_info)),
    }
}

pub enum ReadAccess {
    Global,
    ApiKey,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ReadAccess {
    type Error = anyhow::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let config = request
            .guard::<State<Config>>()
            .await
            .expect("AuthMode was not configured");

        match &config.auth {
            AuthMode::Unauthenticated => Outcome::Success(ReadAccess::Global),
            AuthMode::GithubOAuth => Outcome::Success(ReadAccess::Global),
            AuthMode::ApiKey(key) => match_api_key(request, key, ReadAccess::ApiKey),
            AuthMode::DoubleApiKey { read, .. } => match read {
                None => Outcome::Success(ReadAccess::Global),
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
                match index.is_scope_owner(scope, github_info.id())? {
                    true => true,
                    false => github_info.login().to_lowercase() == scope,
                }
            }
        };

        Ok(has_permission)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for WriteAccess {
    type Error = anyhow::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let config = request
            .guard::<State<Config>>()
            .await
            .expect("AuthMode was not configured");

        match &config.auth {
            AuthMode::Unauthenticated => Outcome::Failure((
                Status::Unauthorized,
                format_err!("Invalid API key for write access"),
            )),
            AuthMode::ApiKey(key) => match_api_key(request, key, WriteAccess::ApiKey),
            AuthMode::DoubleApiKey { write, .. } => {
                match_api_key(request, write, WriteAccess::ApiKey)
            }
            AuthMode::GithubOAuth => verify_github_token(request).await,
        }
    }
}
