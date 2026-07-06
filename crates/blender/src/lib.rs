#![warn(tail_expr_drop_order, clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    reason = "Error variants are self-describing"
)]

use std::{
    io::{Seek, Write},
    path::PathBuf,
    time::Duration,
};

use chromiumoxide::{
    Browser, BrowserConfig,
    cdp::{
        browser_protocol::{
            browser::BrowserContextId,
            page::CaptureScreenshotFormat,
            security::SetIgnoreCertificateErrorsParams,
            target::{CreateBrowserContextParams, CreateTargetParams},
        },
        js_protocol::runtime::EvaluateParams,
    },
    error::CdpError,
};
use futures::stream::StreamExt;
use image::load_from_memory_with_format;
use log::{debug, error, warn};
use thiserror::Error;
use tokio::{sync::RwLock, task::JoinHandle};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Setup failed: {0}")]
    Setup(String),
    #[error("Could not create browser context")]
    CouldNotCreateContext,
    #[error("Internal render error")]
    InternalRender(#[source] CdpError),
    #[error("Invalid URL")]
    InvalidUrl(#[source] url::ParseError),
    #[error("Not found")]
    NotFound,
    #[error("Rendering did not complete within {RENDER_TIMEOUT:?}")]
    Timeout,
    #[error("Image processing error")]
    Image,
    #[error("{0}")]
    Other(String),
}

impl From<image::ImageError> for Error {
    fn from(_: image::ImageError) -> Self {
        Self::Image
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

impl From<CdpError> for Error {
    fn from(err: CdpError) -> Self {
        match err {
            CdpError::LaunchExit(_, stderr) => Self::Setup(format!("{stderr:?}")),
            CdpError::NotFound => Self::NotFound,
            CdpError::Url(e) => Self::InvalidUrl(e),
            x => Self::InternalRender(x),
        }
    }
}

pub struct RenderedImage {
    inner: image::DynamicImage,
}

impl RenderedImage {
    #[must_use]
    pub fn into_grayscaled(self) -> Self {
        Self {
            inner: self.inner.grayscale(),
        }
    }

    pub fn write_as_png<W: Seek + Write>(&self, writer: &mut W) -> Result<(), Error> {
        Ok(self.inner.write_to(writer, image::ImageFormat::Png)?)
    }

    pub fn write_as_qoi<W: Seek + Write>(&self, writer: &mut W) -> Result<(), Error> {
        Ok(self
            .inner
            .to_rgb8()
            .write_to(writer, image::ImageFormat::Qoi)
            .inspect_err(|e| error!("Could not write image: {e:?}"))?)
    }

    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.inner.as_bytes().len()
    }
}

impl From<image::DynamicImage> for RenderedImage {
    fn from(img: image::DynamicImage) -> Self {
        Self { inner: img }
    }
}

/// Upper bound for a whole render (navigation, settling, screenshot). Also
/// the backstop that frees the request if the browser stops responding.
const RENDER_TIMEOUT: Duration = Duration::from_secs(30);
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
/// Grace period after the load event for scripts that draw asynchronously
/// (e.g. charts) before the screenshot is taken.
const SETTLE_DELAY: Duration = Duration::from_millis(200);

struct BrowserState {
    browser: Browser,
    event_task: JoinHandle<()>,
}

impl BrowserState {
    async fn launch(user_dir: Option<PathBuf>) -> Result<Self, Error> {
        let mut config = BrowserConfig::builder()
            .new_headless_mode()
            .window_size(800, 480)
            .arg("--use-skia-font-manager")
            .arg("--disable-partial-raster")
            .arg("--disable-skia-runtime-opts")
            .arg("--deterministic-mode")
            .arg("--font-render-hinting=none")
            .arg("--disable-lcd-text")
            .arg("--disable-font-subpixel-positioning")
            .arg("--use-gl=angle")
            .arg("--use-angle=swiftshader")
            .arg("--enable-unsafe-swiftshader")
            .arg("--allow-insecure-localhost")
            .arg("--ignore-certificate-errors")
            .arg("--test-type")
            .arg("--disable-gpu")
            .arg("--disable-dev-shm-usage")
            .arg("--hide-scrollbars")
            .build()
            .map_err(Error::Setup)?;
        config.user_data_dir = user_dir;
        let (browser, mut handler) = Browser::launch(config).await?;

        let event_task = tokio::task::spawn(async move {
            while let Some(e) = handler.next().await {
                debug!("Got event: {e:?}");
            }
        });
        browser
            .execute(SetIgnoreCertificateErrorsParams::new(true))
            .await
            .map_err(Error::InternalRender)?;

        Ok(Self {
            browser,
            event_task,
        })
    }

    async fn is_responsive(&self) -> bool {
        tokio::time::timeout(HEALTH_CHECK_TIMEOUT, self.browser.version())
            .await
            .is_ok_and(|version| version.is_ok())
    }

    async fn shutdown(&mut self) {
        if let Some(Err(e)) = self.browser.kill().await {
            warn!("Failed to kill browser process: {e}");
        }
        self.event_task.abort();
    }
}

pub struct Instance {
    state: RwLock<BrowserState>,
    user_dir: Option<PathBuf>,
}

impl Instance {
    pub async fn new(user_dir: Option<PathBuf>) -> Result<Self, Error> {
        Ok(Self {
            state: RwLock::new(BrowserState::launch(user_dir.clone()).await?),
            user_dir,
        })
    }

    pub async fn render(&self, url: &str) -> Result<RenderedImage, Error> {
        match self.try_render(url).await {
            Err(err) if !self.is_responsive().await => {
                warn!("Browser is unresponsive after a failed render ({err}); relaunching");
                self.relaunch().await?;
                self.try_render(url).await
            }
            result => result,
        }
    }

    async fn is_responsive(&self) -> bool {
        self.state.read().await.is_responsive().await
    }

    /// Replaces a dead browser with a fresh one. In-flight renders (read
    /// locks) finish first; concurrent relaunch attempts collapse into one.
    async fn relaunch(&self) -> Result<(), Error> {
        let mut state = self.state.write().await;
        if state.is_responsive().await {
            return Ok(());
        }
        // The old process must be gone before launching: two browsers cannot
        // share one user data directory. If the launch fails, the dead state
        // stays in place and the next render triggers another relaunch.
        state.shutdown().await;
        *state = BrowserState::launch(self.user_dir.clone()).await?;
        drop(state);
        Ok(())
    }

    async fn try_render(&self, url: &str) -> Result<RenderedImage, Error> {
        let state = self.state.read().await;
        let context = state
            .browser
            .create_browser_context(CreateBrowserContextParams::default())
            .await?;
        let result = tokio::time::timeout(
            RENDER_TIMEOUT,
            render_in_context(&state.browser, &context, url),
        )
        .await;
        // Dispose unconditionally: a leaked context keeps its page alive in
        // the browser forever, degrading it a little more with every failure.
        if let Err(e) = state.browser.dispose_browser_context(context).await {
            warn!("Failed to dispose browser context: {e}");
        }
        drop(state);
        result.map_err(|_| Error::Timeout)?
    }
}

async fn render_in_context(
    browser: &Browser,
    context: &BrowserContextId,
    url: &str,
) -> Result<RenderedImage, Error> {
    let page = browser
        .new_page(
            CreateTargetParams::builder()
                .url(url)
                .browser_context_id(context.clone())
                .build()?,
        )
        .await?;
    page.wait_for_navigation().await?;
    page.evaluate(await_promise("document.fonts.ready")).await?;
    tokio::time::sleep(SETTLE_DELAY).await;
    // Two rAFs guarantee the frame produced after settling has been painted.
    page.evaluate(await_promise(
        "new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))",
    ))
    .await?;
    let element = page.find_element("html").await?;
    let img = load_from_memory_with_format(
        &element.screenshot(CaptureScreenshotFormat::Png).await?,
        image::ImageFormat::Png,
    )?;
    debug!("Rendered {}x{} image for {url}", img.width(), img.height());
    Ok(RenderedImage::from(img))
}

fn await_promise(expression: &str) -> EvaluateParams {
    EvaluateParams::builder()
        .expression(expression)
        .await_promise(true)
        .build()
        .expect("Hardcoded evaluation only needs an expression, which is set")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_from_image_error() {
        let result: Result<(), Error> =
            image::load_from_memory_with_format(&[], image::ImageFormat::Png)
                .map(|_| ())
                .map_err(Into::into);
        assert!(matches!(result, Err(Error::Image)));
    }

    #[test]
    fn error_from_string() {
        let err: Error = "oops".to_string().into();
        assert!(matches!(err, Error::Other(ref s) if s == "oops"));
    }

    #[test]
    fn error_from_cdp_error_not_found() {
        let err: Error = CdpError::NotFound.into();
        assert!(matches!(err, Error::NotFound));
    }

    #[test]
    fn error_from_cdp_error_url() {
        let parse_err = url::ParseError::EmptyHost;
        let err: Error = CdpError::Url(parse_err).into();
        assert!(matches!(err, Error::InvalidUrl(url::ParseError::EmptyHost)));
    }

    #[test]
    fn error_from_cdp_error_other() {
        let cdp_err = CdpError::from(std::io::Error::other("fail"));
        let err: Error = cdp_err.into();
        assert!(matches!(err, Error::InternalRender(_)));
    }

    #[test]
    fn rendered_image_byte_size() {
        let img = image::DynamicImage::new_rgb8(10, 10);
        let rendered = RenderedImage::from(img);
        assert_eq!(rendered.byte_size(), 300);
    }

    #[test]
    fn rendered_image_into_grayscaled() {
        let img = image::DynamicImage::new_rgb8(10, 10);
        let rendered = RenderedImage::from(img);
        let gray = rendered.into_grayscaled();
        assert_eq!(gray.byte_size(), 100);
    }

    #[test]
    fn rendered_image_write_png() {
        let img = image::DynamicImage::new_rgb8(10, 10);
        let rendered = RenderedImage::from(img);
        let mut buf = std::io::Cursor::new(Vec::new());
        rendered
            .write_as_png(&mut buf)
            .expect("Failed to write PNG");
        assert!(!buf.into_inner().is_empty());
    }

    #[test]
    fn rendered_image_write_qoi() {
        let img = image::DynamicImage::new_rgb8(10, 10);
        let rendered = RenderedImage::from(img);
        let mut buf = std::io::Cursor::new(Vec::new());
        rendered
            .write_as_qoi(&mut buf)
            .expect("Failed to write QOI");
        assert!(!buf.into_inner().is_empty());
    }
}
