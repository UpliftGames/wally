//! Defines a convenient error type for use with Rocket.

use std::io::Cursor;

use rocket::{
    http::{ContentType, Status},
    response::Responder,
    Request, Response,
};
use serde::Serialize;

pub trait ApiErrorContext<T> {
    fn status(self, status: Status) -> Result<T, Error>;
}

impl<T, E> ApiErrorContext<T> for Result<T, E>
where
    E: Into<Error>,
{
    fn status(self, status: Status) -> Result<T, Error> {
        self.map_err(|err| err.into().status(status))
    }
}

pub trait ApiErrorStatus {
    fn status(self, status: Status) -> Error;
}

impl<E> ApiErrorStatus for E
where
    E: Into<Error>,
{
    fn status(self, status: Status) -> Error {
        self.into().status(status)
    }
}

/// Error type returned by most API endpoints. This type has automatic
/// conversions from pretty much any error type and uses an `anyhow::Error`
/// internally.
pub struct Error {
    message: String,
    status: Status,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

impl Error {
    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }
}

impl<E> From<E> for Error
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self {
            message: format!("{:?}", error.into()),
            status: Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, _request: &'r Request<'_>) -> rocket::response::Result<'static> {
        let response = ErrorResponse {
            message: self.message,
        };
        let output = serde_json::to_string(&response).unwrap();

        // TODO: Look at request's `Accept` header and return a different
        // content type if applicable.

        Response::build()
            .sized_body(output.len(), Cursor::new(output))
            .header(ContentType::new("application", "json"))
            .status(self.status)
            .ok()
    }
}
