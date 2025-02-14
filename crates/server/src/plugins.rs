use futures::future::BoxFuture;

use crate::{generator, pages, storage};

pub mod ticktick;

pub enum Plugin {
    Ticktick {
        client: ticktick::Client,
        project: ticktick::Project,
    },
    TestScreen,
}

impl TryFrom<(&String, &toml::Value)> for Plugin {
    type Error = storage::LoadError;

    fn try_from((id, cfg): (&String, &toml::Value)) -> Result<Self, Self::Error> {
        cfg.as_table()
            .map(|cfg| {
                if id == "ticktick" {
                    let project = ticktick::Project::from_toml(
                        cfg.get("project_id")
                            .ok_or(storage::LoadError::InvalidConfig)?,
                    )
                    .ok_or(storage::LoadError::InvalidConfig)?;
                    Ok(Self::Ticktick {
                        client: cfg
                            .try_into()
                            .map_err(|_| storage::LoadError::InvalidConfig)?,
                        project,
                    })
                } else {
                    Err(storage::LoadError::InvalidConfig)
                }
            })
            .transpose()?
            .ok_or(storage::LoadError::InvalidConfig)
    }
}

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
