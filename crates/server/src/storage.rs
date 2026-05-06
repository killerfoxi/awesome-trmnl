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
                content_resource: match &d.content_source {
                    ondisk::ContentSource::Remote(url) => Resource::Remote(url.clone()),
                    ondisk::ContentSource::Local(_) => Resource::self_hosted_content(id),
                },
            })
    }

    pub fn content_generator(
        &self,
        id: &str,
    ) -> Result<&plugins::mashup::Mashup, generator::SetupError> {
        debug!("Trying to find {id}");
        self.devices
            .get(id)
            .inspect(|_| debug!("Found an entry"))
            .and_then(|d| d.content_source.as_local())
            .ok_or(generator::SetupError::Missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(content: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "atrmnl_test_{}_{}.toml",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let mut file = std::fs::File::create(&path).expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write temp config");
        path
    }

    #[tokio::test]
    async fn storage_load_none_mashup() {
        let cfg = r#"
[mydevice]
mashup = { none = "https://example.com/screen" }
plugins = []
"#;
        let path = write_temp_config(cfg);
        let storage = Storage::load(Some(path.clone()))
            .await
            .expect("Failed to load storage");
        std::fs::remove_file(&path).expect("Failed to remove temp file");

        let device = storage.device_by_id("mydevice").expect("Device not found");
        assert_eq!(device.id, "mydevice");
        assert!(matches!(device.content_resource, Resource::Remote(ref url) if url.as_str() == "https://example.com/screen"));
    }

    #[tokio::test]
    async fn storage_load_test_screen_plugin() {
        let cfg = r#"
[mydevice]
mashup = { single = "test" }
plugins = ["test_screen"]
"#;
        let path = write_temp_config(cfg);
        let storage = Storage::load(Some(path.clone()))
            .await
            .expect("Failed to load storage");
        std::fs::remove_file(&path).expect("Failed to remove temp file");

        let device = storage.device_by_id("mydevice").expect("Device not found");
        assert_eq!(device.id, "mydevice");
        assert!(matches!(device.content_resource, Resource::Local(_)));

        let generator = storage.content_generator("mydevice");
        assert!(generator.is_ok());
    }

    #[tokio::test]
    async fn storage_device_not_found() {
        let cfg = r#"
[mydevice]
mashup = { none = "https://example.com" }
plugins = []
"#;
        let path = write_temp_config(cfg);
        let storage = Storage::load(Some(path.clone()))
            .await
            .expect("Failed to load storage");
        std::fs::remove_file(&path).expect("Failed to remove temp file");

        assert!(storage.device_by_id("nonexistent").is_none());
        assert!(storage.content_generator("nonexistent").is_err());
    }
}

mod ondisk {
    use std::{collections::HashMap, fmt::Debug, fs, path::PathBuf, pin::Pin, sync::Arc};

    use log::error;
    use url::Url;

    use crate::plugins::{self, PluginsMap, mashup::Mashup};

    #[derive(Debug)]
    pub enum Error {
        NotFound,
        InvalidConfig,
        LoadConfig(toml::de::Error),
        UnknownPlugin(String),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NotFound => write!(f, "The device file was not found"),
                Self::InvalidConfig => {
                    write!(f, "The device file was loaded, but contained invalid data.")
                }
                Self::LoadConfig(error) => write!(f, "{error}"),
                Self::UnknownPlugin(name) => write!(f, "Unknown plugin: {name}"),
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

    #[derive(serde::Deserialize)]
    struct DeviceConfig {
        mashup: MashupSpec,
        plugins: Vec<plugins::PluginConfig>,
    }

    #[derive(Debug)]
    pub enum ContentSource {
        Remote(Url),
        Local(Mashup),
    }

    impl ContentSource {
        pub const fn as_local(&self) -> Option<&Mashup> {
            match self {
                Self::Local(m) => Some(m),
                Self::Remote(_) => None,
            }
        }
    }

    pub struct Device {
        pub content_source: ContentSource,
        plugins: plugins::PluginsMap,
    }

    impl Debug for Device {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Device")
                .field("plugins", &self.plugins.keys())
                .field("content_source", &self.content_source)
                .finish()
        }
    }

    pub async fn load_local(path: Option<PathBuf>) -> Result<HashMap<String, Device>, Error> {
        let cfg = fs::read_to_string(path.unwrap_or_else(|| {
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/devices.toml"))
        }))
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
            let content_source = match dinfo.mashup {
                MashupSpec::None(url) => ContentSource::Remote(url),
                MashupSpec::Single(source) => ContentSource::Local(Mashup::Single(
                    source.resolve(&plugins).ok_or(Error::UnknownPlugin(source.0))?,
                )),
                MashupSpec::LeftRight { left, right } => {
                    let l = left
                        .resolve(&plugins)
                        .ok_or_else(|| Error::UnknownPlugin(left.0.clone()))?;
                    let r = right
                        .resolve(&plugins)
                        .ok_or_else(|| Error::UnknownPlugin(right.0.clone()))?;
                    ContentSource::Local(Mashup::LeftRight { left: l, right: r })
                }
            };
            devices.insert(id, Device { content_source, plugins });
        }
        Ok(devices)
    }
}
