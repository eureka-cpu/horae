// NOTE: This module requires `toml = "0.8"` in Cargo.toml as a server-only optional dependency.
// It is already wired into the "server" feature flag in Cargo.toml.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use super::{event::AppEvent, manifest::PluginManifest};

/// Holds loaded plugin state. In Phase 2 this is a stub; extism integration comes later.
pub struct PluginRegistry {
    plugins: Mutex<HashMap<String, LoadedPlugin>>,
}

struct LoadedPlugin {
    manifest: PluginManifest,
    // wasm_bytes: Vec<u8>,  // stored for re-instantiation
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
        }
    }

    /// Scan `plugins_dir` for *.wasm files and load their manifests.
    pub async fn load_from_dir(&self, plugins_dir: &Path) -> anyhow::Result<()> {
        if !plugins_dir.exists() {
            tracing::info!("Plugin directory {:?} does not exist, skipping", plugins_dir);
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(plugins_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                let manifest_path = path.with_extension("toml");
                if manifest_path.exists() {
                    let manifest_str = tokio::fs::read_to_string(&manifest_path).await?;
                    match toml::from_str::<PluginManifest>(&manifest_str) {
                        Ok(manifest) => {
                            let name = manifest.plugin.name.clone();
                            tracing::info!("Loaded plugin: {} v{}", name, manifest.plugin.version);
                            self.plugins.lock().unwrap().insert(name, LoadedPlugin { manifest });
                        }
                        Err(e) => tracing::warn!("Failed to parse plugin manifest {:?}: {}", manifest_path, e),
                    }
                }
            }
        }
        Ok(())
    }

    /// Dispatch an event to all subscribed plugins.
    pub async fn dispatch(&self, event: &AppEvent) {
        let hook = event.hook_name();
        let plugins = self.plugins.lock().unwrap();
        for (name, plugin) in plugins.iter() {
            if plugin.manifest.plugin.hooks.iter().any(|h| h == hook) {
                tracing::debug!("Dispatching event '{}' to plugin '{}'", hook, name);
                // TODO: call extism plugin function
            }
        }
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.lock().unwrap().len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
