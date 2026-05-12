use crate::sdk;
use crate::device::Device;
use std::fs;
use std::path::PathBuf;

/// Hardcoded fallback device IDs to try when creating an AVD.
/// We override resolution/DPI via config.ini afterward, so any modern device works.
const FALLBACK_DEVICE_IDS: &[&str] = &["pixel_6_pro", "pixel_6", "pixel_5", "pixel_4", "pixel", "Nexus_5X", "Nexus_5"];

/// Pick a usable device ID from avdmanager. Returns the first match or a fallback.
fn pick_device_id(sdk_path: &PathBuf) -> String {
    if let Ok(defs) = list_device_definitions(sdk_path) {
        for &candidate in FALLBACK_DEVICE_IDS {
            if defs.iter().any(|d| d == candidate) {
                return candidate.to_string();
            }
        }
        if let Some(first) = defs.into_iter().next() {
            return first;
        }
    }
    // Last-resort hardcoded fallback
    "pixel_6_pro".to_string()
}

pub fn create_avd(
    sdk_path: &PathBuf,
    device_id: &str,
    display_name: &str,
    api_level: u8,
    abi: &str,
    tag: &str,
    fingerprint_profile: Option<String>,
    profile_resolution_w: Option<u16>,
    profile_resolution_h: Option<u16>,
    profile_dpi: Option<u16>,
    devices_dir: &PathBuf,
) -> Result<Device, String> {
    let avd_name = format!("enmulator_{}", device_id);
    let avd_path = devices_dir.join(device_id);
    let avdmanager = sdk::get_avdmanager_path(sdk_path);

    let package = format!("system-images;android-{};{};{}", api_level, tag, abi);

    let device_def = pick_device_id(sdk_path);

    let avd_path_str = avd_path.to_str()
        .ok_or("AVD path contains invalid UTF-8")?
        .to_string();

    let output = sdk::sdk_command(&avdmanager)
        .args([
            "create", "avd", "--force",
            "--name", &avd_name,
            "--package", &package,
            "--device", &device_def,
            "--path", &avd_path_str,
        ])
        .output()
        .map_err(|e| format!("avdmanager error: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // Override config.ini with profile resolution/DPI if provided
    if let (Some(w), Some(h), Some(d)) = (profile_resolution_w, profile_resolution_h, profile_dpi) {
        override_config_ini(&avd_path, w, h, d)?;
    }

    Ok(Device {
        id: device_id.to_string(),
        display_name: display_name.to_string(),
        avd_name,
        profile: Some(device_def),
        fingerprint_profile,
        api_level,
        status: "stopped".to_string(),
        port: 0,
        root_enabled: false,
        adb_enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Override hw.lcd.width, hw.lcd.height, hw.lcd.density in the AVD's config.ini
fn override_config_ini(avd_path: &PathBuf, width: u16, height: u16, dpi: u16) -> Result<(), String> {
    let config_path = avd_path.join("config.ini");
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config.ini: {}", e))?;

    let mut new_lines: Vec<String> = Vec::new();
    let mut found_width = false;
    let mut found_height = false;
    let mut found_density = false;

    for line in content.lines() {
        if line.starts_with("hw.lcd.width=") {
            new_lines.push(format!("hw.lcd.width={}", width));
            found_width = true;
        } else if line.starts_with("hw.lcd.height=") {
            new_lines.push(format!("hw.lcd.height={}", height));
            found_height = true;
        } else if line.starts_with("hw.lcd.density=") {
            new_lines.push(format!("hw.lcd.density={}", dpi));
            found_density = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found_width {
        new_lines.push(format!("hw.lcd.width={}", width));
    }
    if !found_height {
        new_lines.push(format!("hw.lcd.height={}", height));
    }
    if !found_density {
        new_lines.push(format!("hw.lcd.density={}", dpi));
    }

    // Use platform-native line endings so the emulator doesn't get confused
    let eol = if cfg!(target_os = "windows") { "\r\n" } else { "\n" };
    fs::write(&config_path, new_lines.join(eol))
        .map_err(|e| format!("Failed to write config.ini: {}", e))?;

    Ok(())
}

pub fn list_device_definitions(sdk_path: &PathBuf) -> Result<Vec<String>, String> {
    let avdmanager = sdk::get_avdmanager_path(sdk_path);
    let output = sdk::sdk_command(&avdmanager)
        .args(["list", "device", "-c"])
        .output()
        .map_err(|e| format!("avdmanager error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}
