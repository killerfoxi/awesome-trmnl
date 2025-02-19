use std::{collections::HashMap, path::PathBuf};

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

pub type LoadError = ondisk::Error;

impl Storage {
    pub async fn load(path: Option<PathBuf>) -> Result<Self, LoadError> {
        let devices = ondisk::load_local(path).await?;
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
                content_resource: match &d.mashup {
                    plugins::mashup::Mashup::None(url) => Resource::Remote(url.clone()),
                    _ => Resource::self_hosted_content(id),
                },
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
            .map(|d| &d.mashup)
            .ok_or(generator::SetupError::Missing)
    }
}

mod ondisk {
    use std::{collections::HashMap, fmt::Debug, fs, path::PathBuf, pin::Pin, sync::Arc};

    use log::error;
    use url::Url;

    use crate::plugins::{self, mashup::Mashup, PluginsMap};

    #[derive(Debug)]
    pub enum Error {
        NotFound,
        InvalidConfig,
        LoadConfig(toml::de::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::NotFound => write!(f, "The device file was not found"),
                Error::InvalidConfig => {
                    write!(f, "The device file was loaded, but contained invalid data.")
                }
                Error::LoadConfig(error) => write!(f, "{error}"),
            }
        }
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Plugin(String);

    impl Plugin {
        pub fn resolve(&self, plugins: &PluginsMap) -> Option<Pin<Arc<plugins::Plugin>>> {
            plugins.get(&self.0).cloned()
        }
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum MashupSpec {
        None(Url),
        Single(Plugin),
        LeftRight { left: Plugin, right: Plugin },
    }

    impl MashupSpec {
        pub fn into_resolved_mashup(self, plugins: &plugins::PluginsMap) -> Mashup {
            match self {
                MashupSpec::None(url) => Mashup::None(url),
                MashupSpec::Single(source) => Mashup::Single(source.resolve(plugins).unwrap()),
                MashupSpec::LeftRight { left, right } => Mashup::LeftRight {
                    left: left.resolve(plugins).unwrap(),
                    right: right.resolve(plugins).unwrap(),
                },
            }
        }
    }

    #[derive(serde::Deserialize)]
    struct DeviceConfig {
        mashup: MashupSpec,
        plugins: Vec<plugins::PluginConfig>,
    }

    pub struct Device {
        pub mashup: Mashup,
        plugins: plugins::PluginsMap,
    }

    impl Debug for Device {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Device")
                .field("plugins", &self.plugins.keys())
                .field("mashup", &self.mashup)
                .finish()
        }
    }

    pub async fn load_local(path: Option<PathBuf>) -> Result<HashMap<String, Device>, Error> {
        let cfg = fs::read_to_string(path.unwrap_or(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/devices.toml"
        ))))
        .map_err(|_| Error::NotFound)?;
        let toml: HashMap<String, DeviceConfig> = toml::from_str(&cfg)
            .inspect_err(|e| error!("{e}"))
            .map_err(Error::LoadConfig)?;
        let mut devices = HashMap::new();
        for (id, dinfo) in toml {
            let mut plugins = HashMap::new();
            for pluginspec in dinfo.plugins {
                let plugin = pluginspec.to_key();
                plugins.insert(
                    plugin.clone(),
                    Arc::pin(
                        plugins::Plugin::new(pluginspec)
                            .await
                            .inspect_err(|_| error!("Creating {plugin} for {id} failed"))?,
                    ),
                );
            }
            devices.insert(
                id,
                Device {
                    mashup: dinfo.mashup.into_resolved_mashup(&plugins),
                    plugins,
                },
            );
        }
        Ok(devices)
    }
}
