use crate::sdk;
use crate::device::Device;
use std::fs;
use std::path::PathBuf;

/// Windows reserved filenames that cannot be used as directory names.
const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul",
    "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
    "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Sanitize a user-provided name into a safe device ID.
/// Keeps only alphanumeric and `-`. Everything else becomes `_`.
/// Rejects Windows reserved filenames (CON, NUL, COM1, etc.) by prefixing `dev_`.
pub fn sanitize_id(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    // Collapse consecutive underscores and trim leading/trailing ones
    let mut result = String::new();
    let mut prev_under = false;
    for c in s.chars() {
        if c == '_' {
            if !prev_under { result.push(c); }
            prev_under = true;
        } else {
            result.push(c);
            prev_under = false;
        }
    }
    let trimmed = result.trim_matches('_').to_string();

    // Prefix Windows reserved names to avoid filesystem errors on Windows
    if WINDOWS_RESERVED.contains(&trimmed.as_str()) {
        format!("dev_{}", trimmed)
    } else {
        trimmed
    }
}

/// Write the `~/.android/avd/<avd_name>.ini` pointer file that avdmanager expects
/// so the emulator can find the AVD by name. Required after a directory-level clone.
pub fn register_avd_at_path(avd_name: &str, avd_dir: &PathBuf) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let android_avd_dir = home.join(".android").join("avd");
    fs::create_dir_all(&android_avd_dir).map_err(|e| e.to_string())?;

    let ini_path = android_avd_dir.join(format!("{}.ini", avd_name));
    let avd_path_str = avd_dir.to_str().ok_or("AVD path is not valid UTF-8")?;
    // Android SDK expects forward slashes even on Windows
    let avd_path_fwd = avd_path_str.replace('\\', "/");
    let content = format!("path={}\npath.rel=avd/{}.avd\n", avd_path_fwd, avd_name);
    fs::write(&ini_path, content).map_err(|e| format!("Failed to write AVD ini: {}", e))?;

    // Update AvdId= in the clone's config.ini so it matches the new name
    let config_ini = avd_dir.join("config.ini");
    if config_ini.exists() {
        if let Ok(content) = fs::read_to_string(&config_ini) {
            let updated: String = content
                .lines()
                .map(|line| {
                    if line.starts_with("AvdId=") {
                        format!("AvdId={}", avd_name)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let _ = fs::write(&config_ini, updated);
        }
    }
    Ok(())
}

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
        writable_system: false,
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

/// Unregister an AVD from avdmanager. Called before deleting the AVD directory
/// to avoid orphaned .ini files in ~/.android/avd/.
pub fn delete_avd(sdk_path: &PathBuf, avd_name: &str) -> Result<(), String> {
    let avdmanager = sdk::get_avdmanager_path(sdk_path);
    let output = sdk::sdk_command(&avdmanager)
        .args(["delete", "avd", "--name", avd_name])
        .output()
        .map_err(|e| format!("avdmanager error: {}", e))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    // Treat "not found" / "does not exist" as success — AVD was never registered
    if stderr.contains("not found") || stderr.contains("does not exist") || stderr.is_empty() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
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
