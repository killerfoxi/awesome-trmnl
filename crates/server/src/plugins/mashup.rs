use std::{pin::Pin, sync::Arc};

use sailfish::TemplateOnce;

use super::Plugin;
use crate::generator;

#[derive(TemplateOnce)]
#[template(path = "mashup/single.stpl")]
struct SingleTemplate {
    inner: String,
}

#[derive(TemplateOnce)]
#[template(path = "mashup/left_right.stpl")]
struct LeftRightTemplate {
    left: String,
    right: String,
}

pub enum Mashup {
    Single(Pin<Arc<Plugin>>),
    LeftRight {
        left: Pin<Arc<Plugin>>,
        right: Pin<Arc<Plugin>>,
    },
}

impl std::fmt::Debug for Mashup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(_) => f.debug_tuple("Single").finish(),
            Self::LeftRight { left: _, right: _ } => f.debug_struct("LeftRight").finish(),
        }
    }
}

impl generator::Content for Mashup {
    fn generate(&self) -> futures::future::BoxFuture<'_, Result<String, generator::Error>> {
        match self {
            Self::Single(p) => Box::pin(async {
                let inner = p.generate().await?;
                Ok(SingleTemplate { inner }.render_once().expect("mashup single template render failed"))
            }),
            Self::LeftRight { left, right } => Box::pin(async {
                let (left, right) = tokio::try_join!(left.generate(), right.generate())?;
                Ok(LeftRightTemplate { left, right }
                    .render_once()
                    .expect("mashup left_right template render failed"))
            }),
        }
    }
}
