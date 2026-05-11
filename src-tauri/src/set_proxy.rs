// set_proxy.rs — per-device proxy config via ADB
// Add to main.rs: mod set_proxy; use set_proxy::ProxyStore; .manage(ProxyStore::new());
// Add command: set_device_proxy

use std::collections::HashMap;
use std::sync::Mutex;

pub struct ProxyStore {
    pub proxies: Mutex<HashMap<String, ProxyConfig>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub enabled: bool,
}

impl ProxyStore {
    pub fn new() -> Self {
        Self { proxies: Mutex::new(HashMap::new()) }
    }

    pub fn set(&self, device_id: &str, config: ProxyConfig) {
        self.proxies.lock().unwrap().insert(device_id.to_string(), config);
    }

    pub fn get(&self, device_id: &str) -> Option<ProxyConfig> {
        self.proxies.lock().unwrap().get(device_id).cloned()
    }

    pub fn remove(&self, device_id: &str) {
        self.proxies.lock().unwrap().remove(device_id);
    }
}
