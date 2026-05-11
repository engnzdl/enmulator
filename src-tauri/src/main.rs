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

use config::Config;
use device::{Device, DeviceStore};
use emulator::EmulatorStore;
use extras::RecordingStore;
use fingerprint::FingerprintProfile;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
fn list_devices(store: tauri::State<Arc<DeviceStore>>) -> Vec<Device> {
    store.list()
}

#[tauri::command]
fn create_device(
    store: tauri::State<Arc<DeviceStore>>,
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

// ── Profiles ──
#[tauri::command]
fn list_profiles() -> Vec<FingerprintProfile> {
    fingerprint::list_profiles(&PathBuf::from("profiles"))
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
    let profiles = fingerprint::list_profiles(&PathBuf::from("profiles"));
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

    let store = Arc::new(DeviceStore::new(devices_dir));
    let emu_store = Arc::new(EmulatorStore::new());
    let rec_store = Arc::new(RecordingStore::new());

    let api_state = Arc::new(api_server::AppState {
        device_store: store.clone(),
        emulator_store: emu_store.clone(),
        config: Arc::new(Mutex::new(cfg.clone())),
    });

    tauri::Builder::default()
        .manage(cfg)
        .manage(config_path)
        .manage(store)
        .manage(emu_store)
        .manage(rec_store)
        .manage(api_state)
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
            start_screen_record,
            stop_screen_record,
            clipboard_sync,
            gps_set,
            logcat_start,
            start_api_server,
            stop_api_server,
            get_config,
            set_sdk_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running enmulator");
}
