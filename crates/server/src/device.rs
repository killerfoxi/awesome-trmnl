use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path},
    http::request::Parts,
};
use url::Url;

use crate::{error::Canonical, resource::Resource, storage};

pub struct Info {
    pub id: String,
    pub content_url: Url,
    pub image_url: Resource,
}

impl<S> FromRequestParts<S> for Info
where
    Arc<storage::Storage>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Canonical;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(id) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| Canonical::InvalidArgument)?;

        let storage = Arc::from_ref(state);
        storage
            .device_by_id(&id)
            .map(|d| Info {
                id: d.id,
                content_url: d.content_resource.fully_qualified_url(),
                image_url: Resource::rendering(&id),
            })
            .ok_or(Canonical::NotFound)
    }
}
