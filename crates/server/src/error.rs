use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use http::StatusCode;

use crate::pages;
pub trait IntoCanonical {
    fn into_canonical(self) -> Canonical;
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Canonical {
    AlreadyExists,
    NotFound,
    PermissionDenied,
    InvalidArgument,
    FailedPrecondition,
    DeadlineExceeded,
    Internal,
    Unknown,
}

impl<E> From<E> for Canonical
where
    E: IntoCanonical,
{
    fn from(value: E) -> Self {
        value.into_canonical()
    }
}

impl IntoResponse for Canonical {
    fn into_response(self) -> Response<Body> {
        match self {
            Self::AlreadyExists => (
                StatusCode::CONFLICT,
                pages::error("Conflicting", "The resource already exists."),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                pages::not_found("The requested resource does not exist."),
            ),
            Self::PermissionDenied => (
                StatusCode::FORBIDDEN,
                pages::error("Nope. Can't do", "The resource is inaccessible."),
            ),
            Self::InvalidArgument => (
                StatusCode::BAD_REQUEST,
                pages::bad_request("Well be better next time."),
            ),
            Self::FailedPrecondition => (
                StatusCode::PRECONDITION_FAILED,
                pages::error("A failed precondition", "huh."),
            ),
            Self::DeadlineExceeded => (
                StatusCode::GATEWAY_TIMEOUT,
                pages::error("Sooo slow", "Did not receive a response in time."),
            ),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::internal_error("Well that's awkward."),
            ),
            Self::Unknown => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::internal_error("This is new for me too!"),
            ),
        }
        .into_response()
    }
}

impl IntoCanonical for blender::Error {
    fn into_canonical(self) -> Canonical {
        match self {
            Self::Setup(_)
            | Self::Other(_)
            | Self::CouldNotCreateContext
            | Self::InternalRender(_)
            | Self::InvalidUrl(_)
            | Self::Image => Canonical::Internal,
            Self::NotFound => Canonical::NotFound,
        }
    }
}
