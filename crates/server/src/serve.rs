use std::{net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    extract::{FromRef, Path, State},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use http::{StatusCode, header};
use itertools::Itertools;
use log::{debug, error, info};
use axum::response::Html;
use sailfish::TemplateOnce;
use rust_embed::Embed;
use serde::Serialize;
use tower_http::trace::TraceLayer;
use url::Url;

use crate::{
    device,
    error::Canonical,
    generator::Content,
    pages,
    resource::{self, Resource},
    storage,
};

enum ImageType {
    Png,
    Qoi,
}

impl ImageType {
    const fn content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Qoi => "image/qoi",
        }
    }

    fn write_image<W: std::io::Seek + std::io::Write>(
        &self,
        img: &blender::RenderedImage,
        writer: &mut W,
    ) -> Result<(), blender::Error> {
        match self {
            Self::Png => img.write_as_png(writer),
            Self::Qoi => img.write_as_qoi(writer),
        }
    }
}

pub async fn embedded_assets(Path(file): Path<String>) -> impl IntoResponse {
    EmbededFile(file)
}

pub async fn serve(
    addr: SocketAddr,
    tls: Option<RustlsConfig>,
    state: ServerState,
    log_requests: bool,
) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/", get(pages::home()))
        .route("/content/{id}", get(screen_content))
        .route("/screen/{id}", get(render_screen_img))
        .route("/preview/{id}", get(preview))
        .route("/assets/{*file}", get(embedded_assets))
        .route("/api/display", get(api_display))
        .with_state(state);
    let app = if log_requests {
        app.layer(
            TraceLayer::new_for_http()
                .on_request(|req: &axum::extract::Request, _span: &tracing::Span| {
                    info!("{:?} {}: {:#?}", req.version(), req.uri(), req.headers());
                })
                .on_response(
                    |resp: &axum::response::Response,
                     _duration: std::time::Duration,
                     _span: &tracing::Span| {
                        info!("Responding: {}; {:#?}", resp.status(), resp.headers());
                    },
                ),
        )
    } else {
        app
    };
    if let Some(cfg) = tls {
        axum_server::bind_rustls(addr, cfg)
            .serve(app.into_make_service())
            .await?;
    } else {
        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await?;
    }
    Ok(())
}

async fn render_screen(
    renderer: &blender::Instance,
    url: Url,
    image_type: ImageType,
) -> axum::response::Result<impl IntoResponse + use<>, Canonical> {
    info!("Requested rendering of: {url}");
    let img = renderer
        .render(url.as_str())
        .await
        .inspect_err(|e| error!("Rendering error: {e:?}"))?;
    let mut writer = std::io::Cursor::new(Vec::with_capacity(img.byte_size()));
    image_type.write_image(&img, &mut writer)?;
    let data = writer.into_inner().into_boxed_slice();
    debug!("Image size: {}", data.len());
    Ok(([(header::CONTENT_TYPE, image_type.content_type())], data))
}

fn determine_image_type(headers: &header::HeaderMap) -> ImageType {
    headers
        .get(http::header::ACCEPT)
        .and_then(|a| a.to_str().ok())
        .filter(|accepting| accepting.split(',').contains("image/qoi"))
        .map_or(ImageType::Png, |_| ImageType::Qoi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determine_image_type_defaults_to_png() {
        let headers = header::HeaderMap::new();
        assert!(matches!(determine_image_type(&headers), ImageType::Png));
    }

    #[test]
    fn determine_image_type_png_explicit() {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            http::header::ACCEPT,
            "image/png".parse().expect("Hardcoded header value is valid"),
        );
        assert!(matches!(determine_image_type(&headers), ImageType::Png));
    }

    #[test]
    fn determine_image_type_qoi() {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            http::header::ACCEPT,
            "image/qoi".parse().expect("Hardcoded header value is valid"),
        );
        assert!(matches!(determine_image_type(&headers), ImageType::Qoi));
    }

    #[test]
    fn determine_image_type_qoi_in_list() {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            http::header::ACCEPT,
            "text/html,image/qoi,image/png"
                .parse()
                .expect("Hardcoded header value is valid"),
        );
        assert!(matches!(determine_image_type(&headers), ImageType::Qoi));
    }

    #[test]
    fn embedded_file_known_asset() {
        let response = EmbededFile("style.css").into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn embedded_file_unknown_asset() {
        let response = EmbededFile("does_not_exist.css").into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[derive(Serialize)]
pub struct ApiResponse {
    image_url: String,
    refresh_rate: u64,
}

#[allow(clippy::unused_async)]
pub async fn api_display(
    State(_storage): State<Arc<storage::Storage>>,
    headers: http::header::HeaderMap,
    device: device::Info,
) -> axum::response::Result<axum::response::Json<ApiResponse>> {
    let mut url = resource::self_url();
    url.set_host(
        headers
            .get(http::header::HOST)
            .map(|h| h.to_str())
            .transpose()
            .map_err(|_| Canonical::InvalidArgument)?,
    )
    .map_err(|_| Canonical::InvalidArgument)?;
    Ok(Json(ApiResponse {
        image_url: Resource::rendering(&device.id)
            .into_remote(url)
            .map_err(|_| Canonical::FailedPrecondition)?
            .fully_qualified_url()
            .as_str()
            .to_owned(),
        refresh_rate: 1800,
    }))
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
        determine_image_type(&headers),
    )
    .await
    .inspect_err(|e| error!("Failed to render image: {e:?}"))
}

#[axum::debug_handler]
async fn screen_content(
    State(storage): State<Arc<storage::Storage>>,
    device: device::Info,
) -> axum::response::Result<Html<String>> {
    debug!("Screen content for {} requested", device.id);
    let content = storage
        .content_generator(&device.id)
        .inspect(|_| debug!("Content found"))?;
    Ok(pages::screen(&content.generate().await?))
}

#[derive(TemplateOnce)]
#[template(path = "serve/preview.stpl")]
struct PreviewTemplate<'a> {
    image_url: &'a str,
}

#[allow(clippy::unused_async)]
async fn preview(_: State<Arc<storage::Storage>>, device: device::Info) -> Html<String> {
    let inner = PreviewTemplate { image_url: device.image_url.as_href() }
        .render_once()
        .expect("preview template render failed");
    pages::index(&inner)
}

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

pub struct EmbededFile<T>(pub T);

impl<T> IntoResponse for EmbededFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Assets::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}

#[derive(FromRef, Clone)]
pub struct ServerState {
    pub renderer: Arc<blender::Instance>,
    pub storage: Arc<storage::Storage>,
}
