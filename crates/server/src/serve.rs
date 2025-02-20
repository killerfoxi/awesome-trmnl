use crate::{device, error::Canonical, generator::Content, pages, storage};

use axum::{
    extract::{FromRef, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use http::header;
use itertools::Itertools;
use log::{debug, error, info};
use maud::{html, Markup};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{services::fs, trace::TraceLayer};
use url::Url;

enum ImageType {
    Png,
    Qoi,
}

impl ImageType {
    fn content_type(&self) -> &'static str {
        match self {
            ImageType::Png => "image/png",
            ImageType::Qoi => "image/qoi",
        }
    }
}

pub(crate) async fn serve(
    listener: TcpListener,
    state: ServerState,
    log_requests: bool,
) -> color_eyre::Result<()> {
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
    let app = if log_requests {
        app.layer(
            TraceLayer::new_for_http()
                .on_request(|req: &axum::extract::Request, _span: &tracing::Span| {
                    info!("{:?} {}: {:#?}", req.version(), req.uri(), req.headers())
                })
                .on_response(
                    |resp: &axum::response::Response,
                     _duration: std::time::Duration,
                     _span: &tracing::Span| {
                        info!("Responding: {}; {:#?}", resp.status(), resp.headers())
                    },
                ),
        )
    } else {
        app
    };
    axum::serve(listener, app).await?;
    Ok(())
}

async fn render_screen(
    renderer: &blender::Instance,
    url: Url,
    image_type: ImageType,
) -> axum::response::Result<impl IntoResponse, Canonical> {
    info!("Requested rendering of: {url}");
    let img = renderer
        .render(url.as_str())
        .await
        .inspect_err(|e| error!("Rendering error: {e:?}"))?; //.map(|i| i.into_grayscaled())?;
    let mut writer = std::io::Cursor::new(Vec::with_capacity(img.byte_size()));
    match image_type {
        ImageType::Png => img.write_as_png(&mut writer)?,
        ImageType::Qoi => img.write_as_qoi(&mut writer)?,
    }
    let data = writer.into_inner().into_boxed_slice();
    debug!("Image size: {}", data.len());
    Ok(([(header::CONTENT_TYPE, image_type.content_type())], data))
}

fn determine_image_type(headers: header::HeaderMap) -> ImageType {
    headers
        .get(http::header::ACCEPT)
        .and_then(|a| {
            a.to_str().ok().map(|accepting| {
                accepting
                    .split(',')
                    .contains("image/qoi")
                    .then_some(ImageType::Qoi)
            })
        })
        .flatten()
        .unwrap_or(ImageType::Png)
}

#[axum::debug_handler]
async fn render_screen_img(
    State(server): State<ServerState>,
    headers: http::header::HeaderMap,
    device: device::Info,
) -> impl IntoResponse {
    render_screen(
        &server.renderer,
        device.content_url,
        determine_image_type(headers),
    )
    .await
    .inspect_err(|e| error!("Failed to render image: {e:?}"))
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
