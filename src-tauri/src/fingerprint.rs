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
    // ── Identity / SIM ──
    pub imei: String,
    pub imei2: String,
    pub meid: String,
    pub phone_number: String,
    pub sim_operator: String,       // MCC+MNC numeric, e.g. "28601"
    pub sim_operator_name: String,  // Display name, e.g. "Turkcell"
    pub sim_country: String,        // ISO country, e.g. "tr"
    pub sim_serial: String,         // ICCID
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

pub fn apply_to_device(
    sdk_path: &PathBuf,
    serial: &str,
    profile: &FingerprintProfile,
) -> Result<(), String> {
    // Build / product identity
    adb_bridge::setprop(sdk_path, serial, "ro.product.brand", &profile.brand)?;
    adb_bridge::setprop(sdk_path, serial, "ro.product.manufacturer", &profile.manufacturer)?;
    adb_bridge::setprop(sdk_path, serial, "ro.product.model", &profile.model)?;
    adb_bridge::setprop(sdk_path, serial, "ro.product.device", &profile.device)?;
    adb_bridge::setprop(sdk_path, serial, "ro.product.name", &profile.device)?;
    adb_bridge::setprop(sdk_path, serial, "ro.build.fingerprint", &profile.fingerprint)?;

    // IMEI / MEID
    adb_bridge::setprop(sdk_path, serial, "persist.radio.imei", &profile.imei)?;
    adb_bridge::setprop(sdk_path, serial, "persist.radio.imei2", &profile.imei2)?;
    adb_bridge::setprop(sdk_path, serial, "persist.radio.meid", &profile.meid)?;

    // Phone number
    adb_bridge::setprop(sdk_path, serial, "gsm.sim.operator.numeric", &profile.sim_operator)?;
    adb_bridge::setprop(sdk_path, serial, "gsm.sim.operator.alpha", &profile.sim_operator_name)?;
    adb_bridge::setprop(sdk_path, serial, "gsm.sim.operator.iso-country", &profile.sim_country)?;
    adb_bridge::setprop(sdk_path, serial, "gsm.operator.numeric", &profile.sim_operator)?;
    adb_bridge::setprop(sdk_path, serial, "gsm.operator.alpha", &profile.sim_operator_name)?;
    adb_bridge::setprop(sdk_path, serial, "gsm.operator.iso-country", &profile.sim_country)?;
    adb_bridge::setprop(sdk_path, serial, "gsm.sim.serial", &profile.sim_serial)?;

    // Phone number display
    adb_bridge::setprop(sdk_path, serial, "gsm.sim.phone_number", &profile.phone_number)?;

    Ok(())
}
