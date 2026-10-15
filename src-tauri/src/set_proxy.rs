// set_proxy.rs — per-device proxy config via ADB
// Add to main.rs: mod set_proxy; use set_proxy::ProxyStore; .manage(ProxyStore::new());
// Add command: set_device_proxy

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ProxyStore {
    pub proxies: Mutex<HashMap<String, ProxyConfig>>,
    path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub enabled: bool,
}

impl ProxyStore {
    /// Load persisted proxies from `path` (if the file exists and parses).
    pub fn new(path: PathBuf) -> Self {
        let proxies = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { proxies: Mutex::new(proxies), path }
    }

    pub fn set(&self, device_id: &str, config: ProxyConfig) {
        self.proxies.lock().unwrap().insert(device_id.to_string(), config);
        self.save();
    }

    pub fn get(&self, device_id: &str) -> Option<ProxyConfig> {
        self.proxies.lock().unwrap().get(device_id).cloned()
    }

    pub fn remove(&self, device_id: &str) {
        self.proxies.lock().unwrap().remove(device_id);
        self.save();
    }

    /// Persist the current map to disk (best-effort; failures are ignored).
    fn save(&self) {
        let data = self.proxies.lock().unwrap().clone();
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&self.path, json);
        }
    }
}
