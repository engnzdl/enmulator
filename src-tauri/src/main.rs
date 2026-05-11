#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod sdk;
mod config;
mod device;
mod avd_manager;
mod emulator;
mod adb_bridge;
mod fingerprint;
mod extras;
mod api_server;
mod set_proxy;
mod bypass;

use config::Config;
use device::{Device, DeviceStore};
use emulator::EmulatorStore;
use extras::RecordingStore;
use fingerprint::FingerprintProfile;
use sdk::SystemImage;
use serde::Serialize;
use set_proxy::{ProxyConfig, ProxyStore};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ── File Explorer ──

#[derive(Debug, Clone, Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
    permissions: String,
}

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

#[tauri::command]
fn list_available_images_cmd(config: tauri::State<Config>) -> Result<Vec<SystemImage>, String> {
    let sdk_path = PathBuf::from(
        config.sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;
    sdk::list_available_images(&sdk_path)
}

#[tauri::command]
fn install_system_image_cmd(app: tauri::AppHandle, config: tauri::State<Config>, package: String) -> Result<(), String> {
    let sdk_path = PathBuf::from(
        config.sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;
    sdk::install_system_image(&app, &sdk_path, &package)
}

// ── Device CRUD ──
#[tauri::command]
fn list_devices(store: tauri::State<Arc<DeviceStore>>) -> Vec<Device> {
    store.list()
}

#[tauri::command]
fn create_device(
    store: tauri::State<Arc<DeviceStore>>,
    config: tauri::State<Config>,
    name: String,
    api_level: u8,
    abi: String,
    tag: String,
    fingerprint_profile: Option<String>,
) -> Result<Device, String> {
    let sdk_path = PathBuf::from(
        config.sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;

    // Look up fingerprint profile for resolution/DPI
    let (res_w, res_h, dpi) = if let Some(ref fp_name) = fingerprint_profile {
        let profiles = fingerprint::list_profiles(&PathBuf::from("../profiles"));
        profiles
            .iter()
            .find(|p| p.name == *fp_name)
            .map(|p| (Some(p.resolution_w), Some(p.resolution_h), Some(p.dpi)))
            .unwrap_or((None, None, None))
    } else {
        (None, None, None)
    };

    let dev = avd_manager::create_avd(
        &sdk_path, &name, &name, api_level, &abi, &tag,
        fingerprint_profile, res_w, res_h, dpi,
        &store.devices_dir,
    )?;
    store.insert(dev.clone());
    Ok(dev)
}

#[tauri::command]
fn delete_device(store: tauri::State<Arc<DeviceStore>>, id: String) -> Result<(), String> {
    store.remove(&id);
    Ok(())
}

#[tauri::command]
fn clone_device(
    store: tauri::State<Arc<DeviceStore>>,
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
        fingerprint_profile: source.fingerprint_profile.clone(),
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
    store: tauri::State<Arc<DeviceStore>>,
    emu_store: tauri::State<Arc<EmulatorStore>>,
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
    store: tauri::State<Arc<DeviceStore>>,
    emu_store: tauri::State<Arc<EmulatorStore>>,
    id: String,
) -> Result<(), String> {
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let dev = store.get(&id).ok_or("Device not found")?;
    emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
    store.update_status(&id, "stopped");
    Ok(())
}

#[tauri::command]
fn check_device_alive(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<bool, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    Ok(EmulatorStore::is_alive(&sdk_path, dev.port))
}

// ── ADB ──
#[tauri::command]
fn adb_shell(config: tauri::State<Config>, store: tauri::State<Arc<DeviceStore>>, id: String, cmd: String) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::shell(&sdk_path, &serial, &cmd)
}

#[tauri::command]
fn install_apk(config: tauri::State<Config>, store: tauri::State<Arc<DeviceStore>>, id: String, apk_path: String) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::install_apk(&sdk_path, &serial, &apk_path)
}

// ── Proxy ──
#[tauri::command]
fn set_device_proxy(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    proxy_store: tauri::State<Arc<ProxyStore>>,
    id: String,
    host: String,
    port: u16,
    enabled: bool,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to set proxy".into());
    }
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);

    if enabled {
        let proxy_cmd = format!("{}:{}", host, port);
        adb_bridge::shell(&sdk_path, &serial, &format!("settings put global http_proxy {}", proxy_cmd))?;
        proxy_store.set(&id, ProxyConfig { host: host.clone(), port, enabled: true });
    } else {
        adb_bridge::shell(&sdk_path, &serial, "settings put global http_proxy :0")?;
        proxy_store.remove(&id);
    }
    Ok(())
}

#[tauri::command]
fn enable_proxy(
    proxy_store: tauri::State<Arc<ProxyStore>>,
    id: String,
) -> Result<Option<ProxyConfig>, String> {
    Ok(proxy_store.get(&id))
}

// ── Profiles ──
#[tauri::command]
fn list_profiles() -> Vec<FingerprintProfile> {
    fingerprint::list_profiles(&PathBuf::from("../profiles"))
}

#[tauri::command]
fn create_profile(profile: FingerprintProfile) -> Result<FingerprintProfile, String> {
    fingerprint::save_profile(&PathBuf::from("../profiles"), &profile);
    Ok(profile)
}

#[tauri::command]
fn delete_profile(name: String) -> Result<(), String> {
    fingerprint::delete_profile(&PathBuf::from("../profiles"), &name)
}

#[tauri::command]
fn apply_profile(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    device_id: String,
    profile_name: String,
) -> Result<(), String> {
    let dev = store.get(&device_id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to apply profile".into());
    }
    let profiles = fingerprint::list_profiles(&PathBuf::from("../profiles"));
    let profile = profiles.into_iter().find(|p| p.name == profile_name).ok_or("Profile not found")?;
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    fingerprint::apply_to_device(&sdk_path, &serial, &profile)
}

// ── Extras: Recording, Clipboard, GPS, Logcat ──
#[tauri::command]
fn start_screen_record(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    rec_store: tauri::State<Arc<RecordingStore>>,
    id: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::start_recording(&sdk_path, &serial, &id, &rec_store)
}

#[tauri::command]
fn stop_screen_record(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    rec_store: tauri::State<Arc<RecordingStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let local_dir = std::env::temp_dir().join("enmulator_recordings");
    extras::stop_recording(&sdk_path, &serial, &id, &rec_store, &local_dir)
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn clipboard_sync(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    direction: String,
    text: Option<String>,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::sync_clipboard(&sdk_path, &serial, &direction, text.as_deref())
}

#[tauri::command]
fn gps_set(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    lat: f64,
    lon: f64,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::set_gps(&sdk_path, &serial, lat, lon)
}

#[tauri::command]
fn logcat_start(
    app: tauri::AppHandle,
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::stream_logcat(&sdk_path, &serial, app).map(|_| ())
}

// ── API Server ──
#[tauri::command]
fn start_api_server(
    api_state: tauri::State<Arc<api_server::AppState>>,
    port: u16,
) -> Result<String, String> {
    api_server::start_api_server(api_state.inner().clone(), port)?;
    Ok(format!("API server started on port {}", port))
}

#[tauri::command]
fn stop_api_server() -> Result<String, String> {
    api_server::stop_api_server()?;
    Ok("API server stopped".to_string())
}

// ── File Explorer ──

#[tauri::command]
fn list_files(config: tauri::State<Config>, store: tauri::State<Arc<DeviceStore>>, id: String, path: String) -> Result<Vec<FileEntry>, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let output = adb_bridge::shell(&sdk_path, &serial, &format!("ls -la {}", path))?;
    let mut entries = Vec::new();
    for line in output.lines() {
        // Skip "total" line
        if line.starts_with("total ") {
            continue;
        }
        // Skip empty lines
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Parse: drwxr-xr-x  2 root root 4096 2024-01-01 12:00 name
        // or:     -rw-r--r--  1 root root  123 2024-01-01 12:00 name
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }
        let perms = parts[0].to_string();
        let is_dir = perms.starts_with('d');
        // Size is the 5th column (0-indexed: 4)
        let size: u64 = parts[4].parse().unwrap_or(0);
        // Name starts at column 8, but may contain spaces if it's the last field
        // Reconstruct name from columns 8+
        let name = parts[8..].join(" ");
        entries.push(FileEntry { name, is_dir, size, permissions: perms });
    }
    Ok(entries)
}

#[tauri::command]
fn pull_file(config: tauri::State<Config>, store: tauri::State<Arc<DeviceStore>>, id: String, remote_path: String, local_path: String) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::pull(&sdk_path, &serial, &remote_path, &local_path)
}

#[tauri::command]
fn push_file(config: tauri::State<Config>, store: tauri::State<Arc<DeviceStore>>, id: String, local_path: String, remote_path: String) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::push(&sdk_path, &serial, &local_path, &remote_path)
}

// ── Snapshot management ──
#[tauri::command]
fn save_snapshot_cmd(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    name: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to save snapshots".into());
    }
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::save_snapshot(&sdk_path, &serial, &name)
}

#[tauri::command]
fn load_snapshot_cmd(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    name: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to load snapshots".into());
    }
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::load_snapshot(&sdk_path, &serial, &name)
}

#[tauri::command]
fn list_snapshots_cmd(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<Vec<String>, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::list_snapshots(&sdk_path, &serial)
}

#[tauri::command]
fn delete_snapshot_cmd(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    name: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to delete snapshots".into());
    }
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::delete_snapshot(&sdk_path, &serial, &name)
}

// ── Batch operations ──

#[derive(Debug, Serialize)]
struct BatchResult {
    success: Vec<String>,
    failed: Vec<BatchFailure>,
}

#[derive(Debug, Serialize)]
struct BatchFailure {
    id: String,
    error: String,
}

#[tauri::command]
async fn batch_start(
    config: tauri::State<'_, Config>,
    store: tauri::State<'_, Arc<DeviceStore>>,
    emu_store: tauri::State<'_, Arc<EmulatorStore>>,
    ids: Vec<String>,
) -> Result<BatchResult, String> {
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let mut result = BatchResult { success: vec![], failed: vec![] };

    for id in &ids {
        let dev = match store.get(id) {
            Some(d) => d,
            None => {
                result.failed.push(BatchFailure { id: id.clone(), error: "Device not found".into() });
                continue;
            }
        };
        if dev.status == "running" {
            result.failed.push(BatchFailure { id: id.clone(), error: "Already running".into() });
            continue;
        }
        match emu_store.start(&sdk_path, &dev.avd_name, false) {
            Ok(port) => {
                store.update_port(id, port);
                store.update_status(id, "running");
                result.success.push(id.clone());
            }
            Err(e) => {
                result.failed.push(BatchFailure { id: id.clone(), error: e });
            }
        }
    }
    Ok(result)
}

#[tauri::command]
async fn batch_stop(
    config: tauri::State<'_, Config>,
    store: tauri::State<'_, Arc<DeviceStore>>,
    emu_store: tauri::State<'_, Arc<EmulatorStore>>,
    ids: Vec<String>,
) -> Result<BatchResult, String> {
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let mut result = BatchResult { success: vec![], failed: vec![] };

    for id in &ids {
        let dev = match store.get(id) {
            Some(d) => d,
            None => {
                result.failed.push(BatchFailure { id: id.clone(), error: "Device not found".into() });
                continue;
            }
        };
        if dev.status != "running" {
            result.failed.push(BatchFailure { id: id.clone(), error: "Not running".into() });
            continue;
        }
        emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
        store.update_status(id, "stopped");
        result.success.push(id.clone());
    }
    Ok(result)
}

#[tauri::command]
async fn batch_delete(
    config: tauri::State<'_, Config>,
    store: tauri::State<'_, Arc<DeviceStore>>,
    emu_store: tauri::State<'_, Arc<EmulatorStore>>,
    ids: Vec<String>,
) -> Result<BatchResult, String> {
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let mut result = BatchResult { success: vec![], failed: vec![] };

    for id in &ids {
        // Stop if running
        if let Some(dev) = store.get(id) {
            if dev.status == "running" {
                emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
            }
        }
        store.remove(id);
        result.success.push(id.clone());
    }
    Ok(result)
}

// ── Root ──

#[tauri::command]
fn toggle_root(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to toggle root".into());
    }
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    
    if dev.root_enabled {
        let adb = sdk::get_adb_path(&sdk_path);
        let _ = std::process::Command::new(&adb)
            .args(["-s", &serial, "unroot"])
            .output();
        store.set_root(&id, false);
        Ok("Root disabled".to_string())
    } else {
        let adb = sdk::get_adb_path(&sdk_path);
        let output = std::process::Command::new(&adb)
            .args(["-s", &serial, "root"])
            .output()
            .map_err(|e| format!("ADB error: {}", e))?;
        
        let out = String::from_utf8_lossy(&output.stdout);
        if out.contains("already running as root") || output.status.success() {
            store.set_root(&id, true);
            Ok("Root enabled".to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("adb root failed: {}. Use a Google APIs system image.", err))
        }
    }
}

// ── Device Templates ──
#[tauri::command]
fn list_device_templates(config: tauri::State<Config>) -> Result<Vec<String>, String> {
    let sdk_path = PathBuf::from(
        config.sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;
    avd_manager::list_device_definitions(&sdk_path)
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

#[tauri::command]
fn bypass_detection(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    bypass::bypass_detection(&sdk_path, &serial)
}

// ── Cert Installer ──

#[tauri::command]
fn install_cert(
    config: tauri::State<Config>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    cert_path: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    let sdk_path = PathBuf::from(config.sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    let adb = sdk::get_adb_path(&sdk_path);

    // Get the cert filename from the path
    let cert_file = std::path::Path::new(&cert_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("Invalid cert path")?;

    // 1. adb root
    let root_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "root"])
        .output()
        .map_err(|e| format!("adb root error: {}", e))?;
    if !root_out.status.success() {
        return Err(format!("adb root failed: {}", String::from_utf8_lossy(&root_out.stderr)));
    }

    // 2. adb remount (makes /system rw)
    let remount_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "remount"])
        .output()
        .map_err(|e| format!("adb remount error: {}", e))?;
    if !remount_out.status.success() {
        return Err(format!("adb remount failed: {}. Device must be rooted.", String::from_utf8_lossy(&remount_out.stderr)));
    }

    // 3. Push cert to /sdcard/
    let push_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "push", &cert_path, &format!("/sdcard/{}", cert_file)])
        .output()
        .map_err(|e| format!("adb push error: {}", e))?;
    if !push_out.status.success() {
        return Err(format!("adb push failed: {}", String::from_utf8_lossy(&push_out.stderr)));
    }

    // 4. Copy cert to /system/etc/security/cacerts/
    let cp_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "cp", &format!("/sdcard/{}", cert_file), &format!("/system/etc/security/cacerts/{}", cert_file)])
        .output()
        .map_err(|e| format!("adb shell cp error: {}", e))?;
    if !cp_out.status.success() {
        return Err(format!("copy cert failed: {}", String::from_utf8_lossy(&cp_out.stderr)));
    }

    // 5. Set permissions 644
    let chmod_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "chmod", "644", &format!("/system/etc/security/cacerts/{}", cert_file)])
        .output()
        .map_err(|e| format!("adb shell chmod error: {}", e))?;
    if !chmod_out.status.success() {
        return Err(format!("chmod failed: {}", String::from_utf8_lossy(&chmod_out.stderr)));
    }

    // 6. Remount /system ro
    let ro_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "mount", "-o", "remount,ro", "/system"])
        .output()
        .map_err(|e| format!("adb shell mount ro error: {}", e))?;
    if !ro_out.status.success() {
        return Err(format!("remount ro failed: {}", String::from_utf8_lossy(&ro_out.stderr)));
    }

    // 7. Clean up /sdcard/ copy
    let _ = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "rm", &format!("/sdcard/{}", cert_file)])
        .output();

    Ok(format!("Certificate {} installed to /system/etc/security/cacerts/", cert_file))
}

fn main() {
    let config_path = PathBuf::from("config.json");
    let mut cfg = config::load(&config_path);

    // Auto-detect SDK on first run if not already configured
    if cfg.sdk_path.is_none() {
        if let Some(detected) = sdk::detect_sdk() {
            cfg.sdk_path = Some(detected.to_string_lossy().to_string());
            config::save(&config_path, &cfg);
        }
    }

    let devices_dir = PathBuf::from(&cfg.devices_dir);

    let store = Arc::new(DeviceStore::new(devices_dir));
    let emu_store = Arc::new(EmulatorStore::new());
    let rec_store = Arc::new(RecordingStore::new());
    let proxy_store = Arc::new(ProxyStore::new());

    let api_state = Arc::new(api_server::AppState {
        device_store: store.clone(),
        emulator_store: emu_store.clone(),
        config: Arc::new(Mutex::new(cfg.clone())),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(cfg)
        .manage(config_path)
        .manage(store)
        .manage(emu_store)
        .manage(rec_store)
        .manage(api_state)
        .manage(proxy_store)
        .invoke_handler(tauri::generate_handler![
            detect_sdk_cmd,
            list_available_images_cmd,
            install_system_image_cmd,
            list_devices,
            create_device,
            delete_device,
            clone_device,
            start_device,
            stop_device,
            check_device_alive,
            batch_start,
            batch_stop,
            batch_delete,
            adb_shell,
            install_apk,
            set_device_proxy,
            enable_proxy,
            list_profiles,
            create_profile,
            delete_profile,
            apply_profile,
            start_screen_record,
            stop_screen_record,
            clipboard_sync,
            gps_set,
            logcat_start,
            start_api_server,
            stop_api_server,
            get_config,
            set_sdk_path,
            list_device_templates,
            list_files,
            pull_file,
            push_file,
            save_snapshot_cmd,
            load_snapshot_cmd,
            list_snapshots_cmd,
            delete_snapshot_cmd,
            toggle_root,
            bypass_detection,
            install_cert,
        ])
        .run(tauri::generate_context!())
        .expect("error while running enmulator");
}
