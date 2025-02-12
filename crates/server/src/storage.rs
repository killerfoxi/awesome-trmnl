use std::collections::HashMap;

use log::debug;

use crate::{generator, plugins, resource::Resource};

#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub content_resource: Resource,
}

pub struct Storage {
    devices: HashMap<String, ondisk::Device>,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            devices: HashMap::from([(
                "test".into(),
                ondisk::Device {
                    plugins: HashMap::from([("test".into(), plugins::Plugin::TestScreen)]),
                    source: ondisk::Source::Plugin("test".into()),
                },
            )]),
        }
    }
}

pub type LoadError = ondisk::Error;

impl Storage {
    pub fn load() -> Result<Self, LoadError> {
        let mut def = Self::default();
        let mut devices = ondisk::load_local()?;
        devices.insert("test".into(), def.devices.remove("test").unwrap());
        debug!("Loaded {} devices", devices.len());
        debug!("Devices: {devices:#?}");
        Ok(Self { devices })
    }

    pub fn device_by_id(&self, id: &str) -> Option<Device> {
        debug!("Device {id} requested.");
        self.devices
            .get(id)
            .inspect(|d| debug!("Found device {d:?}"))
            .map(|d| Device {
                id: id.into(),
                content_resource: d.source.as_resource(id),
            })
    }

    pub fn content_generator(
        &self,
        id: &str,
    ) -> Result<&impl generator::Content, generator::SetupError> {
        debug!("Trying to find {id}");
        self.devices
            .get(id)
            .inspect(|_| debug!("Found an entry"))
            .and_then(|d| d.plugins.get(d.source.plugin_name()))
            .ok_or(generator::SetupError::Missing)
    }
}

mod ondisk {
    use std::{collections::HashMap, fmt::Debug, fs};

    use log::debug;
    use url::Url;

    use crate::{plugins, resource::Resource};

    #[derive(Debug)]
    pub enum Error {
        NotFound,
        InvalidConfig,
    }

    #[derive(Debug)]
    pub enum Source {
        Remote(Url),
        Plugin(String),
    }

    impl Source {
        pub fn as_resource(&self, id: &str) -> Resource {
            match self {
                Source::Remote(url) => Resource::Remote(url.clone()),
                Source::Plugin(_) => Resource::self_hosted_content(id),
            }
        }

        pub fn plugin_name(&self) -> &str {
            match self {
                Source::Remote(_) => panic!("This is a remote resource"),
                Source::Plugin(p) => p,
            }
        }
    }

    impl TryFrom<toml::Value> for Source {
        type Error = Error;

        fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
            value
                .as_str()
                .and_then(|v| {
                    v.strip_prefix("plugin:")
                        .map(|p| Source::Plugin(p.into()))
                        .or_else(|| {
                            v.strip_prefix("remote:")
                                .and_then(|u| Some(Source::Remote(u.parse().ok()?)))
                        })
                })
                .ok_or(Error::InvalidConfig)
        }
    }

    pub struct Device {
        pub plugins: HashMap<String, plugins::Plugin>,
        pub source: Source,
    }

    impl Debug for Device {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Device")
                .field("plugins", &self.plugins.keys())
                .field("source", &self.source)
                .finish()
        }
    }

    pub fn load_local() -> Result<HashMap<String, Device>, Error> {
        let cfg = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/devices.toml"))
            .map_err(|_| Error::NotFound)?;
        let toml: toml::Table = cfg.parse().map_err(|_| Error::InvalidConfig)?;
        toml.into_iter()
            .map(|(id, field)| match field {
                toml::Value::Table(mut data) => {
                    let source = data
                        .remove("source")
                        .map(Source::try_from)
                        .transpose()?
                        .ok_or(Error::InvalidConfig)?;
                    debug!("Found {source:?} for {id}");

                    let plugins = data
                        .iter()
                        .map(|plugin| -> Result<(String, plugins::Plugin), Error> {
                            Ok((plugin.0.clone(), plugins::Plugin::try_from(plugin)?))
                        })
                        .collect::<Result<HashMap<_, _>, Error>>()?;

                    Ok((id, Device { plugins, source }))
                }
                _ => Err(Error::InvalidConfig),
            })
            .collect::<Result<HashMap<_, _>, Error>>()
    }
}
