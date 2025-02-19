use crate::{device, error::Canonical, generator::Content, pages, storage};

use axum::{
    extract::{FromRef, State},
    routing::get,
    Router,
};
use log::{debug, error, info};
use maud::{html, Markup};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::services::fs;
use url::Url;

pub(crate) async fn serve(listener: TcpListener, state: ServerState) -> color_eyre::Result<()> {
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
    axum::serve(listener, app).await?;
    Ok(())
}

async fn render_screen(
    renderer: &blender::Instance,
    url: Url,
) -> axum::response::Result<Box<[u8]>, Canonical> {
    info!("Requested rendering of: {url}");
    let img = renderer
        .render(url.as_str())
        .await
        .inspect_err(|e| error!("Rendering error: {e:?}"))
        .map(|i| i.into_grayscaled())?;
    let mut writer = std::io::Cursor::new(Vec::with_capacity(img.byte_size()));
    img.write_as_png(&mut writer)?;
    Ok(writer.into_inner().into_boxed_slice())
}

#[axum::debug_handler]
async fn render_screen_img(
    State(server): State<ServerState>,
    device: device::Info,
) -> axum::response::Result<Box<[u8]>, Canonical> {
    render_screen(&server.renderer, device.content_url).await
}

#[axum::debug_handler]
async fn screen_content(
    State(storage): State<Arc<storage::Storage>>,
    device: device::Info,
) -> axum::response::Result<Markup> {
    debug!("Screen content for {} requested", device.id);
    let content = storage
        .content_generator(&device.id)
        .inspect(|_| debug!("Content found"))?;
    Ok(pages::screen(content.generate().await?))
}

#[axum::debug_handler]
async fn preview(_: State<Arc<storage::Storage>>, device: device::Info) -> Markup {
    pages::index(html! {
        h1 { "Preview TRMNL screen" }
        img src=(device.image_url.as_href());
    })
}

#[derive(FromRef, Clone)]
pub struct ServerState {
    pub renderer: Arc<blender::Instance>,
    pub storage: Arc<storage::Storage>,
}
