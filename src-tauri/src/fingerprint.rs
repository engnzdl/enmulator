use crate::adb_bridge;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FingerprintProfile {
    pub name: String,
    pub brand: String,
    pub model: String,
    pub manufacturer: String,
    pub device: String,
    pub fingerprint: String,
    pub dpi: u16,
    pub resolution_w: u16,
    pub resolution_h: u16,
    // ── Identity / SIM — optional, default to empty string ──
    #[serde(default)]
    pub imei: String,
    #[serde(default)]
    pub imei2: String,
    #[serde(default)]
    pub meid: String,
    #[serde(default)]
    pub phone_number: String,
    #[serde(default)]
    pub sim_operator: String,
    #[serde(default)]
    pub sim_operator_name: String,
    #[serde(default)]
    pub sim_country: String,
    #[serde(default)]
    pub sim_serial: String,
}

pub fn list_profiles(profiles_dir: &PathBuf) -> Vec<FingerprintProfile> {
    let mut profiles = Vec::new();
    if let Ok(entries) = fs::read_dir(profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(p) = serde_json::from_str::<FingerprintProfile>(&content) {
                        profiles.push(p);
                    }
                }
            }
        }
    }
    profiles
}

pub fn save_profile(profiles_dir: &PathBuf, profile: &FingerprintProfile) {
    fs::create_dir_all(profiles_dir).ok();
    let filename = format!("{}.json", profile.name.to_lowercase().replace(' ', "_"));
    let path = profiles_dir.join(filename);
    let content = serde_json::to_string_pretty(profile).unwrap();
    fs::write(path, content).ok();
}

pub fn delete_profile(profiles_dir: &PathBuf, name: &str) -> Result<(), String> {
    let filename = format!("{}.json", name.to_lowercase().replace(' ', "_"));
    let path = profiles_dir.join(filename);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete profile: {}", e))
    } else {
        Err(format!("Profile '{}' not found", name))
    }
}

/// Apply mutable identity properties to a running device (requires root).
/// ro.product.* properties are immutable (from system image) — NOT applied.
pub fn apply_to_device(
    sdk_path: &PathBuf,
    serial: &str,
    profile: &FingerprintProfile,
) -> Result<(), String> {
    // Restart adbd as root
    let _ = adb_bridge::shell(sdk_path, serial, "root");

    // IMEI / MEID
    if !profile.imei.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "persist.radio.imei", &profile.imei);
    }
    if !profile.imei2.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "persist.radio.imei2", &profile.imei2);
    }
    if !profile.meid.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "persist.radio.meid", &profile.meid);
    }

    // Phone number
    if !profile.phone_number.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.sim.phone_number", &profile.phone_number);
    }

    // Operator
    if !profile.sim_operator.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.sim.operator.numeric", &profile.sim_operator);
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.operator.numeric", &profile.sim_operator);
    }
    if !profile.sim_operator_name.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.sim.operator.alpha", &profile.sim_operator_name);
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.operator.alpha", &profile.sim_operator_name);
    }
    if !profile.sim_country.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.sim.operator.iso-country", &profile.sim_country);
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.operator.iso-country", &profile.sim_country);
    }

    // SIM serial
    if !profile.sim_serial.is_empty() {
        let _ = adb_bridge::setprop(sdk_path, serial, "gsm.sim.serial", &profile.sim_serial);
    }

    Ok(())
}
