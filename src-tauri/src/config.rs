use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub sdk_path: Option<String>,
    pub devices_dir: String,
    pub api_server_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sdk_path: None,
            devices_dir: "devices".to_string(),
            api_server_port: 8080,
        }
    }
}

pub fn load(config_path: &PathBuf) -> Config {
    if config_path.exists() {
        fs::read_to_string(config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        let cfg = Config::default();
        save(config_path, &cfg);
        cfg
    }
}

pub fn save(config_path: &PathBuf, config: &Config) {
    fs::write(config_path, serde_json::to_string_pretty(config).unwrap()).ok();
}
