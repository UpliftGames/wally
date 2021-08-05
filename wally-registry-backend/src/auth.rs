use std::fmt;

use anyhow::{format_err, Context};
use constant_time_eq::constant_time_eq;
use libwally::{package_id::PackageId, package_index::PackageIndex};
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
    GithubOAuth(String),
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
            AuthMode::GithubOAuth(_) => write!(formatter, "Github OAuth"),
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

async fn verify_github_token(
    request: &Request<'_>,
    client_id: &str,
    result: WriteAccess,
) -> Outcome<WriteAccess, anyhow::Error> {
    let token: String = match request.headers().get_one("authorization") {
        Some(key) if key.starts_with("Bearer ") => (key[6..].trim()).to_owned(),
        _ => {
            return Outcome::Failure((Status::Unauthorized, format_err!("Github auth required")));
        }
    };

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("accept", "application/json")
        .header("user-agent", "wally")
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json::<GithubInfo>()
        .await;

    match response {
        Err(err) => Outcome::Failure((Status::Unauthorized, format_err!("Github auth failed"))),
        Ok(github_info) => Outcome::Success(WriteAccess {
            github: Some(github_info),
        }),
    }
}

pub struct ReadAccess {
    _dummy: i32,
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
            AuthMode::Unauthenticated => Outcome::Success(Self { _dummy: 0 }),
            AuthMode::GithubOAuth(_) => Outcome::Success(Self { _dummy: 0 }),
            AuthMode::ApiKey(key) => match_api_key(request, key, Self { _dummy: 0 }),
            AuthMode::DoubleApiKey { read, .. } => match read {
                None => Outcome::Success(Self { _dummy: 0 }),
                Some(key) => match_api_key(request, key, Self { _dummy: 0 }),
            },
        }
    }
}

pub struct WriteAccess {
    github: Option<GithubInfo>,
}

impl WriteAccess {
    pub fn github(&self) -> &Option<GithubInfo> {
        &self.github
    }

    pub fn can_write_package(
        &self,
        package_id: &PackageId,
        index: &PackageIndex,
    ) -> anyhow::Result<()> {
        let scope = package_id.name().scope();
        let package_owners = index.get_scope_owners(&scope)?;

        match self.github() {
            None => Ok(()), // We authenticated using another method
            Some(github_info) => match package_owners {
                None if github_info.login() == scope => {
                    index
                        .add_scope_owner(scope, github_info.id())
                        .context("Could not add owner to scope")?;
                    Ok(())
                }
                None => Err(format_err!("you cannot claim this scope")),
                Some(owners) => match owners.iter().any(|owner| owner == github_info.id()) {
                    true => Ok(()),
                    false => Err(format_err!("you do not own this scope")),
                },
            },
        }
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
            AuthMode::ApiKey(key) => match_api_key(request, key, Self { github: None }),
            AuthMode::DoubleApiKey { write, .. } => {
                match_api_key(request, write, Self { github: None })
            }
            AuthMode::GithubOAuth(client_id) => {
                verify_github_token(request, client_id, Self { github: None }).await
            }
        }
    }
}
