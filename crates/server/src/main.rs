use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use log::{error, info};
use tokio::net::TcpListener;

#[derive(Debug)]
enum Error {
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
    url: &str,
) -> axum::response::Result<Box<[u8]>> {
    info!("Requested rendering of: {url}");
    let img = renderer
        .render(url)
        .await
        .inspect_err(|e| error!("Rendering error: {e:?}"))
        .map(|i| i.into_grayscaled())
        .map_err(Error::from)?;
    let mut writer = std::io::Cursor::new(Vec::with_capacity(img.byte_size()));
    img.write_as_png(&mut writer).map_err(Error::from)?;
    Ok(writer.into_inner().into_boxed_slice())
}

#[axum::debug_handler]
async fn render_test_screen(
    State(state): State<Arc<ServerState>>,
) -> axum::response::Result<Box<[u8]>> {
    render_screen(&state.renderer, "http://localhost:8223/screen/test.html").await
}

struct ServerState {
    renderer: blender::Instance,
}

#[tokio::main]
async fn main() {
    colog::init();
    let state = Arc::new(ServerState {
        renderer: blender::Instance::new().await.unwrap(),
    });
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(r#"<html><body><img src="/screen/test"><body></html>"#) }),
        )
        .route("/screen/test", get(render_test_screen))
        .route(
            "/screen/test.html",
            get(|| async { Html(include_str!("../assets/test.screen.html")) }),
        )
        .with_state(state);
    let listener = TcpListener::bind("0.0.0.0:8223").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
