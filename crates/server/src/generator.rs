use axum::response::IntoResponse;
use futures::future::BoxFuture;
use http::StatusCode;
use maud::Markup;

use crate::pages;

#[derive(Debug)]
pub enum FetchErrorKind {
    Request(StatusCode),
    Network,
    Timeout,
}

#[derive(Debug)]
pub enum Error {
    Fetch {
        kind: FetchErrorKind,
        target: String,
    },
    Misconfigured,
    Unknown,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::Fetch { kind, target } => match kind {
                FetchErrorKind::Request(status_code) => (
                    StatusCode::BAD_GATEWAY,
                    pages::error(
                        "Unexpected response",
                        &format!("Fetching from {target} resulted in {status_code}"),
                    ),
                ),
                FetchErrorKind::Network => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    pages::error(
                        "A network error occurred",
                        "While obtaining content a network error was encountered.",
                    ),
                ),
                FetchErrorKind::Timeout => (
                    StatusCode::GATEWAY_TIMEOUT,
                    pages::error(
                        "Retrieval took too long",
                        "The request to retrieve the content took too long.",
                    ),
                ),
            },
            Error::Misconfigured => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::error(
                    "Misconfigured plugin",
                    "The plugin can't produce content because it's misconfigured.",
                ),
            ),
            Error::Unknown => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::internal_error("It's unclear what happened, but it was not good."),
            ),
        }
        .into_response()
    }
}

#[derive(Debug)]
pub enum SetupError {
    Missing,
}

impl IntoResponse for SetupError {
    fn into_response(self) -> axum::response::Response {
        match self {
            SetupError::Missing => (
                StatusCode::BAD_REQUEST,
                pages::error(
                    "Setup for content incomplete",
                    "There needs to be additional setup before this content can be used.",
                ),
            ),
        }
        .into_response()
    }
}

pub trait Content {
    fn generate(&self) -> BoxFuture<'_, Result<Markup, Error>>;
}
