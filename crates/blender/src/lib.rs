#![warn(tail_expr_drop_order)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

use std::{
    io::{Seek, Write},
    path::PathBuf,
    time::Duration,
};

use chromiumoxide::{
    Browser, BrowserConfig,
    cdp::browser_protocol::target::{CreateBrowserContextParams, CreateTargetParams},
    error::CdpError,
};
use futures::stream::StreamExt;
use image::load_from_memory_with_format;
use log::{debug, error};
use tokio::task::JoinHandle;

#[derive(Debug)]
pub enum Error {
    Setup(String),
    CouldNotCreateContext,
    InternalRender(CdpError),
    InvalidUrl(url::ParseError),
    NotFound,
    Image,
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
            .arg("--font-render-hinting=none")
            .arg("--force-device-scale-factor=1")
            .arg("--allow-insecure-localhost")
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
