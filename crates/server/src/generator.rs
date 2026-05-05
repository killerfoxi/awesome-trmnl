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
    InvalidData,
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

#[allow(clippy::fallible_impl_from, reason = "we know it's a status")]
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let target = err.url().map_or_else(String::default, ToString::to_string);
        if err.is_status() {
            Self::Fetch {
                kind: FetchErrorKind::Request(err.status().unwrap()),
                target,
            }
        } else if err.is_connect() {
            Self::Fetch {
                kind: FetchErrorKind::Network,
                target,
            }
        } else if err.is_timeout() {
            Self::Fetch {
                kind: FetchErrorKind::Timeout,
                target,
            }
        } else if err.is_decode() {
            Self::Fetch {
                kind: FetchErrorKind::InvalidData,
                target,
            }
        } else {
            Self::Unknown
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Fetch { kind, target } => match kind {
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
                FetchErrorKind::InvalidData => (
                    StatusCode::BAD_GATEWAY,
                    pages::error(
                        "Gateway response invalid",
                        "The response from upstream returned invalid data",
                    ),
                ),
            },
            Self::Misconfigured => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::error(
                    "Misconfigured plugin",
                    "The plugin can't produce content because it's misconfigured.",
                ),
            ),
            Self::Unknown => (
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
            Self::Missing => (
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_error_missing_into_response() {
        let resp = SetupError::Missing.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn error_fetch_request_into_response() {
        let err = Error::Fetch {
            kind: FetchErrorKind::Request(StatusCode::NOT_FOUND),
            target: "https://example.com".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn error_fetch_network_into_response() {
        let err = Error::Fetch {
            kind: FetchErrorKind::Network,
            target: "https://example.com".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn error_fetch_timeout_into_response() {
        let err = Error::Fetch {
            kind: FetchErrorKind::Timeout,
            target: "https://example.com".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[test]
    fn error_fetch_invalid_data_into_response() {
        let err = Error::Fetch {
            kind: FetchErrorKind::InvalidData,
            target: "https://example.com".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn error_misconfigured_into_response() {
        let resp = Error::Misconfigured.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn error_unknown_into_response() {
        let resp = Error::Unknown.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

pub trait Content {
    fn generate(&self) -> BoxFuture<'_, Result<Markup, Error>>;
}
