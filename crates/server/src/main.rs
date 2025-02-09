use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode},
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
    InvalidId,
    RenderError,
}

impl From<blender::Error> for Error {
    fn from(_: blender::Error) -> Self {
        Error::RenderError
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response<Body> {
        (StatusCode::INTERNAL_SERVER_ERROR, "Rendering has failed").into_response()
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
    Path(id): Path<String>,
) -> axum::response::Result<Box<[u8]>> {
    render_screen(
        &state.renderer,
        state.render_url.join(&id).map_err(|_| Error::InvalidId)?,
    )
    .await
}

async fn screen_content(Path(id): Path<String>) -> Markup {
    pages::test_screen()
}

async fn preview(Path(id): Path<String>) -> Markup {
    pages::index(html! {
        h1 { "Preview TRMNL screen" }
        img src={"/screen/" (id)};
    })
}

struct ServerState {
    renderer: blender::Instance,
    render_url: Url,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    colog::init();

    let mut url = Url::parse("http://localhost/content/")?;
    url.set_port(Some(8223)).unwrap();
    let state = Arc::new(ServerState {
        renderer: blender::Instance::new().await.unwrap(),
        render_url: url,
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
