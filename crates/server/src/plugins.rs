use std::{collections::HashMap, pin::Pin, sync::Arc};

use futures::future::BoxFuture;

use crate::{generator, pages, storage};

pub mod mashup;
pub mod ticktick;

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfig {
    Ticktick {
        project_id: String,
        auth: ticktick::Auth,
    },
    TestScreen,
}

impl PluginConfig {
    pub fn to_key(&self) -> String {
        match self {
            PluginConfig::Ticktick { .. } => String::from("ticktick"),
            PluginConfig::TestScreen => String::from("test"),
        }
    }
}

pub enum Plugin {
    Ticktick {
        client: ticktick::Client,
        project: ticktick::Project,
    },
    TestScreen,
}

impl TryFrom<PluginConfig> for Plugin {
    type Error = storage::LoadError;

    fn try_from(value: PluginConfig) -> Result<Self, Self::Error> {
        match value {
            PluginConfig::Ticktick { project_id, auth } => Ok(Self::Ticktick {
                client: ticktick::Client::new(auth)
                    .map_err(|_| storage::LoadError::InvalidConfig)?,
                project: project_id.into(),
            }),
            PluginConfig::TestScreen => Ok(Plugin::TestScreen),
        }
    }
}

impl TryInto<(String, Pin<Arc<Plugin>>)> for PluginConfig {
    type Error = storage::LoadError;

    fn try_into(self) -> Result<(String, Pin<Arc<Plugin>>), Self::Error> {
        Ok((self.to_key(), Arc::pin(self.try_into()?)))
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
                    .map_err(|e| e.into())
            }),
        }
    }
}
