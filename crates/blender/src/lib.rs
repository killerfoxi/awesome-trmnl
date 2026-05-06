#![warn(tail_expr_drop_order, clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc, reason = "Error variants are self-describing")]

use std::{
    io::{Seek, Write},
    path::PathBuf,
    time::Duration,
};

use chromiumoxide::{
    Browser, BrowserConfig,
    cdp::browser_protocol::{
        security::SetIgnoreCertificateErrorsParams,
        target::{CreateBrowserContextParams, CreateTargetParams},
    },
    error::CdpError,
};
use futures::stream::StreamExt;
use image::load_from_memory_with_format;
use log::{debug, error};
use thiserror::Error;
use tokio::task::JoinHandle;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Setup failed: {0}")]
    Setup(String),
    #[error("Could not create browser context")]
    CouldNotCreateContext,
    #[error("Internal render error: {0}")]
    InternalRender(CdpError),
    #[error("Invalid URL: {0}")]
    InvalidUrl(url::ParseError),
    #[error("Not found")]
    NotFound,
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

pub struct Instance {
    browser: Browser,
    _event_handle: JoinHandle<()>,
}

impl Instance {
    pub async fn new(user_dir: Option<PathBuf>) -> Result<Self, Error> {
        let mut config = BrowserConfig::builder()
            .new_headless_mode()
            .window_size(800, 480)
            .arg("--use-skia-font-manager")
            .arg("--disable-partial-raster")
            .arg("--disable-skia-runtime-opts")
            .arg("--deterministic-mode")
            .arg("--font-render-hinting=none")
            .arg("--force-device-scale-factor=1")
            .arg("--use-gl=angle")
            .arg("--use-angle=swiftshader")
            .arg("--enable-unsafe-swiftshader")
            .arg("--allow-insecure-localhost")
            .arg("--ignore-certificate-errors")
            .arg("--test-type")
            .arg("--disable-gpu")
            .build()
            .map_err(Error::Setup)?;
        config.user_data_dir = user_dir;
        let (browser, mut handler) = Browser::launch(config).await?;

        let handle = tokio::task::spawn(async move {
            while let Some(e) = handler.next().await {
                debug!("Got event: {e:?}");
            }
        });
        browser
            .execute(SetIgnoreCertificateErrorsParams::new(true))
            .await
            .map_err(|e| Error::InternalRender(e))?;

        Ok(Self {
            browser,
            _event_handle: handle,
        })
    }

    pub async fn render(&self, url: &str) -> Result<RenderedImage, Error> {
        let context = self
            .browser
            .create_browser_context(CreateBrowserContextParams::default())
            .await?;
        let screen = self
            .browser
            .new_page(
                CreateTargetParams::builder()
                    .url(url)
                    .browser_context_id(context.clone())
                    .build()?,
            )
            .await?;
        tokio::time::sleep(Duration::from_millis(1300)).await;
        let element = screen.find_element("html").await?;
        let img = load_from_memory_with_format(
            &element
                .screenshot(
                    chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png,
                )
                .await?,
            image::ImageFormat::Png,
        )?;
        self.browser.dispose_browser_context(context).await?;
        Ok(RenderedImage::from(img.unsharpen(1.5, 112)))
    }
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
