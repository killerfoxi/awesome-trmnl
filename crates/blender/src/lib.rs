use std::{
    io::{Seek, Write},
    time::Duration,
};

use chromiumoxide::{
    cdp::browser_protocol::{
        accessibility::EventLoadComplete,
        page::EventLoadEventFired,
        target::{CreateBrowserContextParams, CreateTargetParams},
    },
    error::CdpError,
    handler::viewport::Viewport,
    page::ScreenshotParams,
    Browser, BrowserConfig,
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
}

impl From<image::ImageError> for Error {
    fn from(_: image::ImageError) -> Self {
        Self::Image
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
    pub async fn new() -> Result<Self, Error> {
        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .new_headless_mode()
                .viewport(Some(Viewport {
                    width: 800,
                    height: 480,
                    device_scale_factor: Some(1.0),
                    emulating_mobile: false,
                    is_landscape: true,
                    has_touch: false,
                }))
                .build()
                .map_err(Error::Setup)?,
        )
        .await?;

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
                    .build()
                    .unwrap(),
            )
            .await?;
        tokio::time::sleep(Duration::from_millis(800)).await;
        let img = load_from_memory_with_format(
            &screen.screenshot(ScreenshotParams::default()).await?,
            image::ImageFormat::Png,
        )?;
        self.browser.dispose_browser_context(context).await?;
        Ok(RenderedImage::from(img))
    }
}
