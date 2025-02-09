use std::sync::Arc;

use axum::{
    body::Body,
    extract::{FromRef, FromRequestParts, Path, State},
    http::{request::Parts, Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use log::{error, info};
use maud::{html, Markup};
use tokio::net::TcpListener;
use url::Url;

mod pages;

#[derive(Debug)]
enum Error {
    Render,
}

impl From<blender::Error> for Error {
    fn from(_: blender::Error) -> Self {
        Error::Render
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response<Body> {
        match self {
            Error::Render => (
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::internal_error(html! { p { "The rendering has failed" } }),
            ),
        }
        .into_response()
    }
}

#[derive(Debug)]
enum DeviceError {
    MissingId,
    InvalidId,
    NotFound,
}

impl From<url::ParseError> for DeviceError {
    fn from(_: url::ParseError) -> Self {
        Self::InvalidId
    }
}

impl IntoResponse for DeviceError {
    fn into_response(self) -> axum::response::Response {
        match self {
            DeviceError::InvalidId => (
                StatusCode::BAD_REQUEST,
                pages::bad_request(html! { p { "The device id was invalid"} }),
            ),
            DeviceError::NotFound => (
                StatusCode::NOT_FOUND,
                pages::not_found(html! { p { "The device is unknown" } }),
            ),
            DeviceError::MissingId => (
                StatusCode::BAD_REQUEST,
                pages::bad_request(html! { p { "Forgot the device id, eh?" } }),
            ),
        }
        .into_response()
    }
}

struct DeviceInfo {
    id: String,
    content_url: Url,
    image_url: Url,
}

impl<S> FromRequestParts<S> for DeviceInfo
where
    Arc<ServerState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = DeviceError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(id) = Path::<String>::from_request_parts(parts, state)
            .await
            .map_err(|_| DeviceError::MissingId)?;
        let state = Arc::from_ref(state);
        if id != "test" {
            return Err(DeviceError::NotFound);
        }
        Ok(DeviceInfo {
            content_url: state.content_url_base.join(&id)?,
            image_url: state.render_url_base.join(&id)?,
            id,
        })
    }
}

async fn render_screen(
    renderer: &blender::Instance,
    url: Url,
) -> axum::response::Result<Box<[u8]>> {
    info!("Requested rendering of: {url}");
    let img = renderer
        .render(url.as_str())
        .await
        .inspect_err(|e| error!("Rendering error: {e:?}"))
        .map(|i| i.into_grayscaled())
        .map_err(Error::from)?;
    let mut writer = std::io::Cursor::new(Vec::with_capacity(img.byte_size()));
    img.write_as_png(&mut writer).map_err(Error::from)?;
    Ok(writer.into_inner().into_boxed_slice())
}

async fn render_screen_img(
    State(state): State<Arc<ServerState>>,
    device: DeviceInfo,
) -> axum::response::Result<Box<[u8]>> {
    render_screen(&state.renderer, device.content_url).await
}

async fn screen_content(device: DeviceInfo) -> axum::response::Result<Markup, Error> {
    if device.id == "test" {
        return Ok(pages::test_screen());
    }
    unimplemented!("this is not implemented yet");
}

async fn preview(device: DeviceInfo) -> Markup {
    pages::index(html! {
        h1 { "Preview TRMNL screen" }
        img src=(device.image_url.path());
    })
}

struct ServerState {
    renderer: blender::Instance,
    render_url_base: Url,
    content_url_base: Url,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    colog::init();

    let mut url = Url::parse("http://localhost/")?;
    url.set_port(Some(8223)).unwrap();
    let state = Arc::new(ServerState {
        renderer: blender::Instance::new().await.unwrap(),
        render_url_base: url.join("screen/").unwrap(),
        content_url_base: url.join("content/").unwrap(),
    });
    let app = Router::new()
        .route(
            "/",
            get(pages::index(html! {
                h1 { "Welcome to Awesome TRMNL." }
                p { "Do you have a TRMNL device? Point it at me." }
                p { "Or see a " a href="/preview/test" { "test preview" } "."}
            })),
        )
        .route("/content/{id}", get(screen_content))
        .route("/screen/{id}", get(render_screen_img))
        .route("/preview/{id}", get(preview))
        .with_state(state);
    let listener = TcpListener::bind("0.0.0.0:8223").await?;
    info!(
        "Successfully started listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;
    Ok(())
}
