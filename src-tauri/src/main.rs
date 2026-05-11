#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod sdk;
mod config;
mod device;
mod avd_manager;
mod emulator;
mod adb_bridge;
mod fingerprint;

use config::Config;
use device::{Device, DeviceStore};
use emulator::EmulatorStore;
use fingerprint::FingerprintProfile;
use std::path::PathBuf;
use tauri::Manager;

// ── SDK ──
#[tauri::command]
fn detect_sdk_cmd(config: tauri::State<Config>) -> Result<String, String> {
    if let Some(ref path) = config.sdk_path {
        return Ok(path.clone());
    }
    sdk::detect_sdk()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or("SDK not found".into())
}

// ── Device CRUD ──
#[tauri::command]
fn list_devices(store: tauri::State<DeviceStore>) -> Vec<Device> {
    store.list()
}

#[tauri::command]
fn create_device(
    store: tauri::State<DeviceStore>,
    config: tauri::State<Config>,
    name: String,
    profile: String,
    api_level: u8,
) -> Result<Device, String> {
    let sdk_path = PathBuf::from(
        config.sdk_path.as_ref().ok_or("SDK not configured")?
    );
    let dev = avd_manager::create_avd(
        &sdk_path, &name, &name, api_level, &profile, &store.devices_dir,
    )?;
    store.insert(dev.clone());
    Ok(dev)
}

#[tauri::command]
fn delete_device(store: tauri::State<DeviceStore>, id: String) -> Result<(), String> {
    store.remove(&id);
    Ok(())
}

#[tauri::command]
fn clone_device(
    store: tauri::State<DeviceStore>,
    source_id: String,
    target_name: String,
) -> Result<Device, String> {
    let source = store.get(&source_id).ok_or("Source not found")?;
    let target_id = target_name.to_lowercase().replace(' ', "_");
    let src_dir = store.devices_dir.join(&source_id);
    let dst_dir = store.devices_dir.join(&target_id);

    // Copy AVD directory
    fn copy_dir(src: &PathBuf, dst: &PathBuf) -> Result<(), String> {
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let dest = dst.join(entry.file_name());
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "snapshots" {
                    copy_dir(&entry.path(), &dest)?;
                }
            } else {
                std::fs::copy(entry.path(), &dest).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
    copy_dir(&src_dir, &dst_dir)?;

    let cloned = Device {
        id: target_id.clone(),
        display_name: target_name,
        avd_name: format!("enmulator_{}", target_id),
        profile: source.profile.clone(),
        api_level: source.api_level,
        status: "stopped".to_string(),
        port: 0,
        root_enabled: source.root_enabled,
        adb_enabled: source.adb_enabled,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.insert(cloned.clone());
    Ok(cloned)
}

// ── Emulator lifecycle ──
#[tauri::command]
fn start_device(
    config: tauri::State<Config>,
    store: tauri::State<DeviceStore>,
    emu_store: tauri::State<EmulatorStore>,
    id: String,
    headless: bool,
) -> Result<u16, String> {
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let dev = store.get(&id).ok_or("Device not found")?;
    let port = emu_store.start(&sdk_path, &dev.avd_name, headless)?;
    store.update_port(&id, port);
    store.update_status(&id, "running");
    Ok(port)
}

#[tauri::command]
fn stop_device(
    config: tauri::State<Config>,
    store: tauri::State<DeviceStore>,
    emu_store: tauri::State<EmulatorStore>,
    id: String,
) -> Result<(), String> {
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let dev = store.get(&id).ok_or("Device not found")?;
    emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
    store.update_status(&id, "stopped");
    Ok(())
}

// ── ADB ──
#[tauri::command]
fn adb_shell(config: tauri::State<Config>, store: tauri::State<DeviceStore>, id: String, cmd: String) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::shell(&sdk_path, &serial, &cmd)
}

#[tauri::command]
fn install_apk(config: tauri::State<Config>, store: tauri::State<DeviceStore>, id: String, apk_path: String) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::install_apk(&sdk_path, &serial, &apk_path)
}

// ── Profiles ──
#[tauri::command]
fn list_profiles() -> Vec<FingerprintProfile> {
    fingerprint::list_profiles(&PathBuf::from("profiles"))
}

#[tauri::command]
fn apply_profile(
    config: tauri::State<Config>,
    store: tauri::State<DeviceStore>,
    device_id: String,
    profile_name: String,
) -> Result<(), String> {
    let dev = store.get(&device_id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to apply profile".into());
    }
    let profiles = fingerprint::list_profiles(&PathBuf::from("profiles"));
    let profile = profiles.into_iter().find(|p| p.name == profile_name).ok_or("Profile not found")?;
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    fingerprint::apply_to_device(&sdk_path, &serial, &profile)
}

// ── Config ──
#[tauri::command]
fn get_config(config: tauri::State<Config>) -> Config {
    config.inner().clone()
}

#[tauri::command]
fn set_sdk_path(config: tauri::State<Config>, config_path_state: tauri::State<PathBuf>, path: String) -> Result<(), String> {
    let mut cfg = config.inner().clone();
    cfg.sdk_path = Some(path);
    config::save(&config_path_state, &cfg);
    Ok(())
}

fn main() {
    let config_path = PathBuf::from("config.json");
    let cfg = config::load(&config_path);
    let devices_dir = PathBuf::from(&cfg.devices_dir);
    let store = DeviceStore::new(devices_dir);
    let emu_store = EmulatorStore::new();

    tauri::Builder::default()
        .manage(cfg)
        .manage(config_path)
        .manage(store)
        .manage(emu_store)
        .invoke_handler(tauri::generate_handler![
            detect_sdk_cmd,
            list_devices,
            create_device,
            delete_device,
            clone_device,
            start_device,
            stop_device,
            adb_shell,
            install_apk,
            list_profiles,
            apply_profile,
            get_config,
            set_sdk_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running enmulator");
}
