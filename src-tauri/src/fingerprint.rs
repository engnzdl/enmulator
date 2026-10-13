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
/// Returns a per-field summary: which props applied and which failed.
pub fn apply_to_device(
    sdk_path: &PathBuf,
    serial: &str,
    profile: &FingerprintProfile,
) -> Result<String, String> {
    // Restart adbd as root
    let _ = adb_bridge::shell(sdk_path, serial, "root");

    // (label, prop, value) — only non-empty fields are attempted
    let mut attempts: Vec<(&str, &str, &str)> = Vec::new();
    if !profile.imei.is_empty() {
        attempts.push(("IMEI", "persist.radio.imei", profile.imei.as_str()));
    }
    if !profile.imei2.is_empty() {
        attempts.push(("IMEI2", "persist.radio.imei2", profile.imei2.as_str()));
    }
    if !profile.meid.is_empty() {
        attempts.push(("MEID", "persist.radio.meid", profile.meid.as_str()));
    }
    if !profile.phone_number.is_empty() {
        attempts.push(("Phone number", "gsm.sim.phone_number", profile.phone_number.as_str()));
    }
    if !profile.sim_operator.is_empty() {
        attempts.push(("Operator numeric", "gsm.sim.operator.numeric", profile.sim_operator.as_str()));
        attempts.push(("Operator numeric (global)", "gsm.operator.numeric", profile.sim_operator.as_str()));
    }
    if !profile.sim_operator_name.is_empty() {
        attempts.push(("Operator name", "gsm.sim.operator.alpha", profile.sim_operator_name.as_str()));
        attempts.push(("Operator name (global)", "gsm.operator.alpha", profile.sim_operator_name.as_str()));
    }
    if !profile.sim_country.is_empty() {
        attempts.push(("SIM country", "gsm.sim.operator.iso-country", profile.sim_country.as_str()));
        attempts.push(("Operator country", "gsm.operator.iso-country", profile.sim_country.as_str()));
    }
    if !profile.sim_serial.is_empty() {
        attempts.push(("SIM serial", "gsm.sim.serial", profile.sim_serial.as_str()));
    }

    let mut applied = 0usize;
    let mut failed: Vec<String> = Vec::new();
    for (label, prop, value) in &attempts {
        match adb_bridge::setprop(sdk_path, serial, prop, value) {
            Ok(()) => applied += 1,
            Err(e) => failed.push(format!("{}: {}", label, e)),
        }
    }

    if applied == 0 && !failed.is_empty() {
        return Err(format!("Identity apply failed: {}", failed.join("; ")));
    }
    if failed.is_empty() {
        Ok(format!("Applied {} identity properties", applied))
    } else {
        Ok(format!(
            "Applied {}/{} identity properties. Failed: {}",
            applied,
            applied + failed.len(),
            failed.join("; ")
        ))
    }
}
