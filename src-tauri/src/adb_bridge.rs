use crate::sdk;
use std::path::PathBuf;

pub fn shell(sdk_path: &PathBuf, serial: &str, cmd: &str) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "shell", cmd])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn setprop(sdk_path: &PathBuf, serial: &str, key: &str, value: &str) -> Result<(), String> {
    let escaped = value.replace('\'', "'\\''");
    shell(sdk_path, serial, &format!("setprop {} '{}'", key, escaped)).map(|_| ())
}

/// Returns true if Magisk (with `su`) is available on the device.
/// When Magisk is installed, `su -c 'magisk -v'` prints a version string.
pub fn has_magisk(sdk_path: &PathBuf, serial: &str) -> bool {
    shell(sdk_path, serial, "su -c 'magisk -v'")
        .map(|o| o.trim().chars().any(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

/// Set a property via Magisk `resetprop` — works on any system image,
/// including read-only `ro.*` props that plain `setprop` cannot touch.
pub fn magisk_resetprop(sdk_path: &PathBuf, serial: &str, key: &str, value: &str) -> Result<(), String> {
    let v = value.replace('"', "\\\"");
    shell(sdk_path, serial, &format!("su -c 'magisk resetprop {} \"{}\"'", key, v)).map(|_| ())
}

/// Run a command as root via Magisk `su`.
pub fn su_shell(sdk_path: &PathBuf, serial: &str, cmd: &str) -> Result<String, String> {
    let escaped = cmd.replace('\'', "'\\''");
    shell(sdk_path, serial, &format!("su -c '{}'", escaped))
}

pub fn install_apk(sdk_path: &PathBuf, serial: &str, apk_path: &str) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "install", "-r", apk_path])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn push(sdk_path: &PathBuf, serial: &str, local: &str, remote: &str) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "push", local, remote])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn pull(sdk_path: &PathBuf, serial: &str, remote: &str, local: &str) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "pull", remote, local])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

