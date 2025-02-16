use std::{pin::Pin, sync::Arc};

use maud::html;
use url::Url;

use super::Plugin;
use crate::generator;

pub enum Mashup {
    None(Url),
    Single(Pin<Arc<Plugin>>),
    LeftRight {
        left: Pin<Arc<Plugin>>,
        right: Pin<Arc<Plugin>>,
    },
}

impl std::fmt::Debug for Mashup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None(u) => f.debug_tuple("None").field(u).finish(),
            Self::Single(_) => f.debug_tuple("Single").finish(),
            Self::LeftRight { left: _, right: _ } => f.debug_struct("LeftRight").finish(),
        }
    }
}

impl generator::Content for Mashup {
    fn generate(&self) -> futures::future::BoxFuture<'_, Result<maud::Markup, generator::Error>> {
        match self {
            Mashup::None(_) => panic!("Can't generate content for remotes"),
            Mashup::Single(p) => Box::pin(async {
                Ok(html! {
                    div ."view view--full" {
                        (p.generate().await?)
                    }
                })
            }),
            Mashup::LeftRight { left, right } => Box::pin(async {
                Ok(html! {
                    div ."mashup mashup--1Lx1R" {
                        div ."view view--half_vertical" {
                            (left.generate().await?)
                        }
                        div ."view view--half_vertical" {
                            (right.generate().await?)
                        }
                    }
                })
            }),
        }
    }
}
