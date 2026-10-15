use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub sdk_path: Option<String>,
    pub devices_dir: String,
    pub api_server_port: u16,
    pub default_headless: bool,
    pub auto_start_api: bool,
    pub default_api_level: u8,
    pub default_abi: String,
    pub default_tag: String,
    #[serde(default)]
    pub api_token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sdk_path: None,
            devices_dir: "devices".to_string(),
            api_server_port: 8080,
            default_headless: false,
            auto_start_api: false,
            default_api_level: 34,
            default_abi: "x86_64".to_string(),
            default_tag: "google_apis".to_string(),
            api_token: String::new(),
        }
    }
}

/// Generate a random 64-char hex token for REST API authentication.
fn generate_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn load(config_path: &PathBuf) -> Config {
    let mut cfg = if config_path.exists() {
        match fs::read_to_string(config_path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(cfg) => cfg,
                Err(e) => {
                    // Corrupt config — back it up and fall back to defaults rather
                    // than silently dropping the user's settings.
                    let backup = config_path.with_extension("json.corrupt");
                    let _ = fs::copy(config_path, &backup);
                    eprintln!(
                        "WARNING: config.json is corrupt ({}). Backed up to {:?}, using defaults.",
                        e, backup
                    );
                    Config::default()
                }
            },
            Err(e) => {
                eprintln!("WARNING: cannot read config.json ({}). Using defaults.", e);
                Config::default()
            }
        }
    } else {
        Config::default()
    };
    // Backfill a random API token on first run (or for older configs without one)
    if cfg.api_token.is_empty() {
        cfg.api_token = generate_token();
        save(config_path, &cfg);
    }
    cfg
}

pub fn save(config_path: &PathBuf, config: &Config) {
    fs::write(config_path, serde_json::to_string_pretty(config).unwrap()).ok();
}
