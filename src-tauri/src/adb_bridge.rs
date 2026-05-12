use crate::sdk;
use std::path::PathBuf;
use std::process::Command;

pub fn shell(sdk_path: &PathBuf, serial: &str, cmd: &str) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = Command::new(&adb)
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
    shell(sdk_path, serial, &format!("setprop {} {}", key, value)).map(|_| ())
}

pub fn install_apk(sdk_path: &PathBuf, serial: &str, apk_path: &str) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = Command::new(&adb)
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
    Command::new(&adb)
        .args(["-s", serial, "push", local, remote])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    Ok(())
}

pub fn pull(sdk_path: &PathBuf, serial: &str, remote: &str, local: &str) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = Command::new(&adb)
        .args(["-s", serial, "pull", remote, local])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

