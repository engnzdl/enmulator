use crate::adb_bridge;
use crate::sdk;
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
                    // Strip UTF-8 BOM — Windows tools (PowerShell Out-File, some editors) add it.
                    let content = content.strip_prefix('\u{FEFF}').unwrap_or(&content);
                    if let Ok(p) = serde_json::from_str::<FingerprintProfile>(content) {
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

/// Derived identity fields parsed from an Android build fingerprint
/// (`brand/product/device:release/id/incremental:type/tags`).
struct DerivedBuildProps {
    product_name: String,
    version_release: String,
    build_id: String,
    version_incremental: String,
    build_type: String,
    build_tags: String,
}

fn parse_fingerprint(fp: &str) -> Option<DerivedBuildProps> {
    let parts: Vec<&str> = fp.split('/').collect();
    if parts.len() != 6 {
        return None;
    }
    let device_release: Vec<&str> = parts[2].split(':').collect();
    let incremental_type: Vec<&str> = parts[4].split(':').collect();
    if device_release.len() < 2 || incremental_type.len() < 2 {
        return None;
    }
    Some(DerivedBuildProps {
        product_name: parts[1].to_string(),
        version_release: device_release[1].to_string(),
        build_id: parts[3].to_string(),
        version_incremental: incremental_type[0].to_string(),
        build_type: incremental_type[1].to_string(),
        build_tags: parts[5].to_string(),
    })
}

/// Build the full boot-time `ro.*` property list for `emulator -prop` injection.
/// Only identity properties that do NOT map to hardware HALs are emitted, so the
/// guest keeps its real SDK/hardware/ABI — spoofing those breaks standard AVDs.
/// All `ro.build.*` fields are derived from the fingerprint string, so the 66
/// profile JSONs need no new fields.
pub fn boot_props(profile: &FingerprintProfile) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    let mut push = |k: &str, v: &str| {
        if !v.is_empty() {
            props.push((k.to_string(), v.to_string()));
        }
    };

    push("ro.product.brand", &profile.brand);
    push("ro.product.model", &profile.model);
    push("ro.product.manufacturer", &profile.manufacturer);
    push("ro.product.device", &profile.device);
    push("ro.build.fingerprint", &profile.fingerprint);

    if let Some(d) = parse_fingerprint(&profile.fingerprint) {
        push("ro.product.name", &d.product_name);
        push("ro.build.version.release", &d.version_release);
        push("ro.build.version.release_or_codename", &d.version_release);
        push("ro.build.id", &d.build_id);
        push("ro.build.version.incremental", &d.version_incremental);
        push("ro.build.type", &d.build_type);
        push("ro.build.tags", &d.build_tags);
    }

    props
}

/// Map the fingerprint's Android release to its API level (Android 10..15 → 29..35).
/// Used to keep the system image's API level consistent with the spoofed
/// fingerprint — a mismatch between SDK_INT and the fingerprint release is a
/// detection signal, so the image is chosen to match the profile.
pub fn api_level(profile: &FingerprintProfile) -> Option<u8> {
    let release = parse_fingerprint(&profile.fingerprint)?.version_release;
    match release.as_str() {
        "10" => Some(29),
        "11" => Some(30),
        "12" => Some(31),
        "12L" => Some(32),
        "13" => Some(33),
        "14" => Some(34),
        "15" => Some(35),
        "16" => Some(36),
        _ => None,
    }
}

/// Persist the full spoofed identity into /system/build.prop so it is present
/// from first boot on every subsequent launch (no runtime resetprop needed).
/// Requires a writable /system (`-writable-system` + `adb root` + `adb remount`).
/// Returns the number of properties written.
pub fn patch_build_prop_identity(
    sdk_path: &PathBuf,
    serial: &str,
    profile: &FingerprintProfile,
) -> Result<usize, String> {
    let props = boot_props(profile);
    let raw = adb_bridge::shell(sdk_path, serial, "cat /system/build.prop")?;
    let content = raw.replace("\r\n", "\n").replace("\r", "\n");

    let mut new_lines: Vec<String> = Vec::new();
    let mut handled: Vec<String> = Vec::new();
    let mut patched = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();
        let mut replaced = false;
        for (key, value) in &props {
            if trimmed.starts_with(&format!("{}=", key)) {
                new_lines.push(format!("{}={}", key, value));
                handled.push(key.clone());
                patched += 1;
                replaced = true;
                break;
            }
        }
        if !replaced {
            new_lines.push(line.to_string());
        }
    }

    // Append any identity props not already present in build.prop.
    for (key, value) in &props {
        if !handled.contains(key) {
            new_lines.push(format!("{}={}", key, value));
            patched += 1;
        }
    }

    let new_content = new_lines.join("\n");
    let adb = sdk::get_adb_path(sdk_path);
    let tmp_local = std::env::temp_dir().join(format!(
        "enmulator_build_{}.prop",
        serial.replace([':', '/', '\\'], "_")
    ));
    std::fs::write(&tmp_local, new_content.as_bytes()).map_err(|e| e.to_string())?;
    sdk::new_command(&adb)
        .args(["-s", serial, "push", &tmp_local.to_string_lossy(), "/sdcard/build.prop"])
        .output()
        .map_err(|e| e.to_string())?;
    adb_bridge::shell(
        sdk_path,
        serial,
        "cp /sdcard/build.prop /system/build.prop && chmod 644 /system/build.prop && rm /sdcard/build.prop",
    )?;
    let _ = std::fs::remove_file(&tmp_local);
    Ok(patched)
}

/// Root + remount + persist identity to build.prop (best-effort wrapper).
pub fn persist_identity(
    sdk_path: &PathBuf,
    serial: &str,
    profile: &FingerprintProfile,
) -> Result<usize, String> {
    let _ = adb_bridge::shell(sdk_path, serial, "root");
    let _ = sdk::new_command(&sdk::get_adb_path(sdk_path))
        .args(["-s", serial, "remount"])
        .output();
    let writable = adb_bridge::shell(sdk_path, serial, "test -w /system && echo 1 || echo 0")
        .map(|o| o.trim() == "1")
        .unwrap_or(false);
    if !writable {
        return Err("system not writable (adb remount failed)".into());
    }
    patch_build_prop_identity(sdk_path, serial, profile)
}

/// Apply mutable identity properties to a running device (requires root).
/// ro.product.* properties are immutable (from system image) — NOT applied.
/// Returns a per-field summary: which props applied and which failed.
pub fn apply_to_device(
    sdk_path: &PathBuf,
    serial: &str,
    profile: &FingerprintProfile,
) -> Result<String, String> {
    // Restart adbd as root (google_apis images only; Magisk `su` is the fallback)
    let _ = adb_bridge::shell(sdk_path, serial, "root");
    let use_magisk = adb_bridge::has_magisk(sdk_path, serial);

    // (label, prop, value) — only non-empty fields are attempted
    let mut attempts: Vec<(&str, String, String)> = Vec::new();
    let mut push = |label: &'static str, prop: &str, value: &str| {
        if !value.is_empty() {
            attempts.push((label, prop.to_string(), value.to_string()));
        }
    };

    // Mutable identity props (plain setprop; reset on reboot)
    push("IMEI", "persist.radio.imei", &profile.imei);
    push("IMEI2", "persist.radio.imei2", &profile.imei2);
    push("MEID", "persist.radio.meid", &profile.meid);
    push("Phone number", "gsm.sim.phone_number", &profile.phone_number);
    push("Operator numeric", "gsm.sim.operator.numeric", &profile.sim_operator);
    push("Operator numeric (global)", "gsm.operator.numeric", &profile.sim_operator);
    push("Operator name", "gsm.sim.operator.alpha", &profile.sim_operator_name);
    push("Operator name (global)", "gsm.operator.alpha", &profile.sim_operator_name);
    push("SIM country", "gsm.sim.operator.iso-country", &profile.sim_country);
    push("Operator country", "gsm.operator.iso-country", &profile.sim_country);
    push("SIM serial", "gsm.sim.serial", &profile.sim_serial);

    // Full ro.* identity — derived from the fingerprint via boot_props().
    // Immutable on stock images; Magisk resetprop overrides them in memory.
    if use_magisk {
        for (prop, value) in boot_props(profile) {
            attempts.push(("Identity", prop, value));
        }
    }

    let mut applied = 0usize;
    let mut failed: Vec<String> = Vec::new();
    for (label, prop, value) in &attempts {
        let result = if use_magisk {
            adb_bridge::magisk_resetprop(sdk_path, serial, prop, value)
        } else {
            adb_bridge::setprop(sdk_path, serial, prop, value)
        };
        match result {
            Ok(()) => applied += 1,
            Err(e) => failed.push(format!("{}: {}", label, e)),
        }
    }

    // Persist the identity into /system/build.prop so it survives reboot (best-effort).
    let persist_note = match persist_identity(sdk_path, serial, profile) {
        Ok(n) => format!("; persisted {} props to build.prop", n),
        Err(_) => String::new(),
    };

    if applied == 0 && !failed.is_empty() {
        return Err(format!("Identity apply failed: {}", failed.join("; ")));
    }
    if failed.is_empty() {
        Ok(format!("Applied {} identity properties{}", applied, persist_note))
    } else {
        Ok(format!(
            "Applied {}/{} identity properties{}. Failed: {}",
            applied,
            applied + failed.len(),
            persist_note,
            failed.join("; ")
        ))
    }
}
