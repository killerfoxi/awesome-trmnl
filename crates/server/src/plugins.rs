use std::{collections::HashMap, pin::Pin, sync::Arc};

use futures::future::BoxFuture;
use weather::Detail;

use crate::{generator, pages, storage};

pub mod mashup;
pub mod ticktick;
pub mod weather;

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfig {
    Ticktick {
        project_id: String,
        auth: ticktick::Auth,
    },
    Weather {
        location: String,
        #[serde(default)]
        detail: Detail,
    },
    TestScreen,
}

impl PluginConfig {
    pub fn to_key(&self) -> String {
        match self {
            PluginConfig::Ticktick { .. } => String::from("ticktick"),
            PluginConfig::TestScreen => String::from("test"),
            PluginConfig::Weather { .. } => String::from("weather"),
        }
    }
}

pub enum Plugin {
    Ticktick {
        client: ticktick::Client,
        project: ticktick::Project,
    },
    Weather {
        client: weather::Client,
    },
    TestScreen,
}

impl Plugin {
    pub async fn new(value: PluginConfig) -> Result<Self, storage::LoadError> {
        match value {
            PluginConfig::Ticktick { project_id, auth } => Ok(Self::Ticktick {
                client: ticktick::Client::new(auth)
                    .map_err(|_| storage::LoadError::InvalidConfig)?,
                project: project_id.into(),
            }),
            PluginConfig::TestScreen => Ok(Plugin::TestScreen),
            PluginConfig::Weather { location, detail } => Ok(Self::Weather {
                client: weather::Client::new(location, detail)
                    .await
                    .map_err(|_| storage::LoadError::InvalidConfig)?,
            }),
        }
    }
}

pub type PluginsMap = HashMap<String, Pin<Arc<Plugin>>>;

impl generator::Content for Plugin {
    fn generate(&self) -> BoxFuture<'_, Result<maud::Markup, generator::Error>> {
        match self {
            Plugin::TestScreen => Box::pin(async { Ok(pages::test_screen()) }),
            Plugin::Ticktick { client, project } => Box::pin(async {
                client
                    .fetch_and_display(project.clone())
                    .await
                    .map_err(std::convert::Into::into)
            }),
            Plugin::Weather { client } => Box::pin(async { client.fetch_and_display().await }),
        }
    }
}
