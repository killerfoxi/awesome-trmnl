use futures::future::BoxFuture;
use http::StatusCode;

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
        use generator::{Error, FetchErrorKind};

        match self {
            Plugin::TestScreen => Box::pin(async { Ok(pages::test_screen()) }),
            Plugin::Ticktick { client, project } => Box::pin(async {
                client
                    .fetch_and_display(project.clone())
                    .await
                    .map_err(|e| {
                        let target = client.endpoint().for_project_data(project).to_string();
                        match e {
                            ticktick::FetchError::Timeout => Error::Fetch {
                                kind: FetchErrorKind::Timeout,
                                target,
                            },
                            ticktick::FetchError::Connection => Error::Fetch {
                                kind: FetchErrorKind::Network,
                                target,
                            },
                            ticktick::FetchError::InvalidRequest => Error::Fetch {
                                kind: FetchErrorKind::Request(StatusCode::BAD_REQUEST),
                                target,
                            },
                            ticktick::FetchError::PermissionDenied => Error::Fetch {
                                kind: FetchErrorKind::Request(StatusCode::FORBIDDEN),
                                target,
                            },
                            ticktick::FetchError::NotFound => Error::Fetch {
                                kind: FetchErrorKind::Request(StatusCode::NOT_FOUND),
                                target,
                            },
                            ticktick::FetchError::Unauthenticated => Error::Fetch {
                                kind: FetchErrorKind::Request(StatusCode::UNAUTHORIZED),
                                target,
                            },
                            ticktick::FetchError::Other => Error::Unknown,
                        }
                    })
            }),
        }
    }
}
