use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub id: String,
    pub display_name: String,
    pub avd_name: String,
    pub profile: Option<String>,
    pub fingerprint_profile: Option<String>,
    pub api_level: u8,
    pub status: String,
    pub port: u16,
    pub root_enabled: bool,
    pub adb_enabled: bool,
    pub created_at: String,
}

pub struct DeviceStore {
    pub devices: Mutex<HashMap<String, Device>>,
    pub devices_dir: PathBuf,
    pub data_path: PathBuf,
}

impl DeviceStore {
    pub fn new(devices_dir: PathBuf) -> Self {
        fs::create_dir_all(&devices_dir).ok();
        let data_path = devices_dir.join("devices.json");
        let devices = if data_path.exists() {
            fs::read_to_string(&data_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self { devices: Mutex::new(devices), devices_dir, data_path }
    }

    fn save_to_disk(&self) {
        let devices = self.devices.lock().unwrap();
        let json = serde_json::to_string_pretty(&*devices).unwrap();
        fs::write(&self.data_path, json).ok();
    }

    pub fn list(&self) -> Vec<Device> {
        self.devices.lock().unwrap().values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<Device> {
        self.devices.lock().unwrap().get(id).cloned()
    }

    pub fn insert(&self, device: Device) {
        self.devices.lock().unwrap().insert(device.id.clone(), device);
        self.save_to_disk();
    }

    pub fn remove(&self, id: &str) {
        self.devices.lock().unwrap().remove(id);
        self.save_to_disk();
        let dir = self.devices_dir.join(id);
        fs::remove_dir_all(dir).ok();
    }

    pub fn update_status(&self, id: &str, status: &str) {
        if let Some(dev) = self.devices.lock().unwrap().get_mut(id) {
            dev.status = status.to_string();
        }
        self.save_to_disk();
    }

    pub fn update_port(&self, id: &str, port: u16) {
        if let Some(dev) = self.devices.lock().unwrap().get_mut(id) {
            dev.port = port;
        }
        self.save_to_disk();
    }

    pub fn set_root(&self, id: &str, rooted: bool) {
        if let Some(dev) = self.devices.lock().unwrap().get_mut(id) {
            dev.root_enabled = rooted;
        }
        self.save_to_disk();
    }
}
