use std::collections::HashMap;

use futures::future::BoxFuture;
use maud::Markup;

use crate::{
    device::{self, GenerationError},
    pages, plugins,
    resource::Resource,
};

#[derive(Clone)]
pub struct Device {
    pub id: String,
    pub content_resource: Resource,
}

pub struct Storage {
    devices: HashMap<String, Device>,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            devices: HashMap::from([
                (
                    "test".into(),
                    Device {
                        id: "test".into(),
                        content_resource: Resource::self_hosted_content("test"),
                    },
                ),
                (
                    "ticktick".into(),
                    Device {
                        id: "ticktick".into(),
                        content_resource: Resource::self_hosted_content("ticktick"),
                    },
                ),
            ]),
        }
    }
}

impl Storage {
    pub fn device_by_id(&self, id: &str) -> Option<Device> {
        self.devices.get(id).cloned()
    }

    pub fn content_generator(
        &self,
        device: &device::Info,
    ) -> BoxFuture<'static, Result<Markup, GenerationError>> {
        match device.id.as_str() {
            "test" => Box::pin(async { Ok(pages::test_screen()) }),
            "ticktick" => Box::pin(async {
                let project = plugins::ticktick::Project::from(String::from("<TBFI>"));
                let client =
                    plugins::ticktick::Client::new("<TBFI>").map_err(|_| GenerationError)?;
                client
                    .fetch_and_display(project)
                    .await
                    .map_err(|_| GenerationError)
            }),
            _ => Box::pin(async { Ok(pages::empty_screen()) }),
        }
    }
}
