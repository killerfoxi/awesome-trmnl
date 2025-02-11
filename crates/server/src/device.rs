use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
};
use maud::html;
use url::Url;

use crate::{pages, resource::Resource, ServerState};

#[derive(Debug)]
pub enum Error {
    MissingId,
    InvalidId,
    NotFound,
}

impl From<url::ParseError> for Error {
    fn from(_: url::ParseError) -> Self {
        Self::InvalidId
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::InvalidId => (
                StatusCode::BAD_REQUEST,
                pages::bad_request(html! { p { "The device id was invalid"} }),
            ),
            Error::NotFound => (
                StatusCode::NOT_FOUND,
                pages::not_found(html! { p { "The device is unknown" } }),
            ),
            Error::MissingId => (
                StatusCode::BAD_REQUEST,
                pages::bad_request(html! { p { "Forgot the device id, eh?" } }),
            ),
        }
        .into_response()
    }
}

pub struct GenerationError;

pub struct Info {
    pub id: String,
    pub content_url: Url,
    pub image_url: Resource,
}

impl<S> FromRequestParts<S> for Info
where
    Arc<ServerState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(id) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| Error::MissingId)?;

        let state = Arc::from_ref(state);
        state
            .storage
            .device_by_id(&id)
            .map(|d| Info {
                id: d.id,
                content_url: d.content_resource.fully_qualified_url(),
                image_url: Resource::rendering(&id),
            })
            .ok_or(Error::NotFound)
    }
}
