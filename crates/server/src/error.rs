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
            Canonical::AlreadyExists => (
                StatusCode::CONFLICT,
                pages::error("Conflicting", "The resource already exists."),
            ),
            Canonical::NotFound => (
                StatusCode::NOT_FOUND,
                pages::not_found("The requested resource does not exist."),
            ),
            Canonical::PermissionDenied => (
                StatusCode::FORBIDDEN,
                pages::error("Nope. Can't do", "The resource is inaccessible."),
            ),
            Canonical::InvalidArgument => (
                StatusCode::BAD_REQUEST,
                pages::bad_request("Well be better next time."),
            ),
            Canonical::FailedPrecondition => (
                StatusCode::PRECONDITION_FAILED,
                pages::error("A failed precondition", "huh."),
            ),
            Canonical::DeadlineExceeded => (
                StatusCode::GATEWAY_TIMEOUT,
                pages::error("Sooo slow", "Did not receive a response in time."),
            ),
            Canonical::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::internal_error("Well that's awkward."),
            ),
            Canonical::Unknown => (
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
            blender::Error::Setup(_)
            | blender::Error::Other(_)
            | blender::Error::CouldNotCreateContext
            | blender::Error::InternalRender(_)
            | blender::Error::InvalidUrl(_)
            | blender::Error::Image => Canonical::Internal,
            blender::Error::NotFound => Canonical::NotFound,
        }
    }
}
