use std::fmt;

use anyhow::format_err;
use constant_time_eq::constant_time_eq;
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
    Unauthenticated,
}

impl fmt::Debug for AuthMode {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthMode::ApiKey(_) => write!(formatter, "API key"),
            AuthMode::DoubleApiKey { read: _, write: _ } => write!(formatter, "double API key"),
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
            AuthMode::ApiKey(key) => match_api_key(request, key, Self { _dummy: 0 }),
            AuthMode::DoubleApiKey { read, write: _ } => match read {
                None => Outcome::Success(Self { _dummy: 0 }),
                Some(key) => match_api_key(request, key, Self { _dummy: 0 }),
            },
        }
    }
}

pub struct WriteAccess {
    _dummy: i32,
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
            AuthMode::ApiKey(key) => match_api_key(request, key, Self { _dummy: 0 }),
            AuthMode::DoubleApiKey { read: _, write } => {
                match_api_key(request, write, Self { _dummy: 0 })
            }
        }
    }
}
