use std::path::PathBuf;

use futures::future::BoxFuture;
use serde_json::Value;

use crate::{generator, storage};

pub struct WasmPlugin {
    manifest: extism::Manifest,
    config: Value,
}

impl WasmPlugin {
    pub fn new(path: PathBuf, config: Value) -> Result<Self, storage::LoadError> {
        if !path.exists() {
            log::error!("WASM plugin not found at {}", path.display());
            return Err(storage::LoadError::InvalidConfig);
        }
        let manifest = extism::Manifest::new([extism::Wasm::file(path)]).with_allowed_host("*");
        Ok(Self { manifest, config })
    }
}

impl generator::Content for WasmPlugin {
    fn generate(&self) -> BoxFuture<'_, Result<String, generator::Error>> {
        let manifest = self.manifest.clone();
        let config = self.config.to_string();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let mut plugin = extism::Plugin::new(&manifest, [], true)
                    .map_err(|e| generator::Error::Wasm(e.to_string()))?;
                plugin
                    .call::<&str, String>("generate", &config)
                    .map_err(|e| generator::Error::Wasm(e.to_string()))
            })
            .await
            .map_err(|_| generator::Error::Unknown)?
        })
    }
}
