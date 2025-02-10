use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use log::{error, info};
use maud::{html, Markup};
use tokio::net::TcpListener;
use tower_http::services::fs;
use url::Url;

mod device;
mod pages;
mod plugins;
mod resource;
mod storage;

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
    device: device::Info,
) -> axum::response::Result<Box<[u8]>> {
    render_screen(&state.renderer, device.content_url.fully_qualified_url()).await
}

async fn screen_content(device: device::Info) -> axum::response::Result<Markup, Error> {
    if device.id == "test" {
        return Ok(pages::test_screen());
    }
    if device.id == "ticktick" {
        return Ok(pages::screen(plugins::ticktick::content()));
    }
    unimplemented!("this is not implemented yet");
}

async fn preview(device: device::Info) -> Markup {
    pages::index(html! {
        h1 { "Preview TRMNL screen" }
        img src=(device.image_url.as_href());
    })
}

pub struct ServerState {
    pub renderer: blender::Instance,
    pub storage: storage::Storage,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    colog::init();

    let state = Arc::new(ServerState {
        renderer: blender::Instance::new().await.unwrap(),
        storage: storage::Storage,
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
        .nest_service("/assets", fs::ServeDir::new("assets"))
        .with_state(state);
    let listener = TcpListener::bind("0.0.0.0:8223").await?;
    info!(
        "Successfully started listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app).await?;
    Ok(())
}
