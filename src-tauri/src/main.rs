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
mod paths;

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
fn detect_sdk_cmd(config: tauri::State<Arc<Mutex<Config>>>) -> Result<String, String> {
    if let Some(ref path) = config.lock().unwrap().sdk_path {
        return Ok(path.clone());
    }
    sdk::detect_sdk()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or("SDK not found".into())
}

#[tauri::command]
fn list_available_images_cmd(config: tauri::State<Arc<Mutex<Config>>>) -> Result<Vec<SystemImage>, String> {
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;
    sdk::list_available_images(&sdk_path)
}

#[tauri::command]
fn install_system_image_cmd(app: tauri::AppHandle, config: tauri::State<Arc<Mutex<Config>>>, package: String) -> Result<(), String> {
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
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
    config: tauri::State<Arc<Mutex<Config>>>,
    name: String,
    api_level: u8,
    abi: String,
    tag: String,
    fingerprint_profile: Option<String>,
) -> Result<Device, String> {
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;

    // Look up fingerprint profile for resolution/DPI
    let (res_w, res_h, dpi) = if let Some(ref fp_name) = fingerprint_profile {
        let profiles = fingerprint::list_profiles(&paths::profiles_dir());
        profiles
            .iter()
            .find(|p| p.name == *fp_name)
            .map(|p| (Some(p.resolution_w), Some(p.resolution_h), Some(p.dpi)))
            .unwrap_or((None, None, None))
    } else {
        (None, None, None)
    };

    let device_id = avd_manager::sanitize_id(&name);
    if device_id.is_empty() {
        return Err("Device name must contain at least one alphanumeric character".into());
    }
    // Reject if ID already taken
    if store.get(&device_id).is_some() {
        return Err(format!("A device with the name '{}' already exists", name));
    }

    let dev = avd_manager::create_avd(
        &sdk_path, &device_id, &name, api_level, &abi, &tag,
        fingerprint_profile, res_w, res_h, dpi,
        &store.devices_dir,
    )?;
    store.insert(dev.clone());
    Ok(dev)
}

#[tauri::command]
fn delete_device(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    emu_store: tauri::State<Arc<EmulatorStore>>,
    id: String,
) -> Result<(), String> {
    if let Some(dev) = store.get(&id) {
        if let Some(sdk_str) = config.lock().unwrap().sdk_path.clone() {
            let sdk_path = PathBuf::from(&sdk_str);
            // Stop emulator if still running
            if dev.status == "running" {
                emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
            }
            // Unregister from avdmanager to avoid orphaned .ini files
            let _ = avd_manager::delete_avd(&sdk_path, &dev.avd_name);
        }
    }
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
    let target_id = avd_manager::sanitize_id(&target_name);
    if target_id.is_empty() {
        return Err("Target name must contain at least one alphanumeric character".into());
    }
    if store.get(&target_id).is_some() {
        return Err(format!("A device named '{}' already exists", target_name));
    }
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

    let new_avd_name = format!("enmulator_{}", target_id);
    // Register the cloned AVD with avdmanager so the emulator can find it by name
    avd_manager::register_avd_at_path(&new_avd_name, &dst_dir)?;

    let cloned = Device {
        id: target_id.clone(),
        display_name: target_name,
        avd_name: new_avd_name,
        profile: source.profile.clone(),
        fingerprint_profile: source.fingerprint_profile.clone(),
        api_level: source.api_level,
        status: "stopped".to_string(),
        port: 0,
        root_enabled: false,
        adb_enabled: source.adb_enabled,
        writable_system: false,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.insert(cloned.clone());
    Ok(cloned)
}

// ── Emulator lifecycle ──
#[tauri::command]
fn start_device(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    emu_store: tauri::State<Arc<EmulatorStore>>,
    id: String,
    headless: bool,
) -> Result<u16, String> {
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let dev = store.get(&id).ok_or("Device not found")?;

    if dev.status == "running" {
        return Err("Device is already running".into());
    }

    let profile = dev.fingerprint_profile.as_ref().and_then(|name| {
        fingerprint::list_profiles(&paths::profiles_dir())
            .into_iter()
            .find(|p| &p.name == name)
    });

    let port = emu_store.start(&sdk_path, &dev.avd_name, headless, profile.as_ref())?;
    store.update_port(&id, port);
    store.update_status(&id, "running");

    // Apply identity (IMEI, operator, phone) after boot via adb root + setprop
    if let Some(fp) = profile {
        let sdk = sdk_path.clone();
        std::thread::spawn(move || {
            let serial = format!("emulator-{}", port);
            let adb = sdk::get_adb_path(&sdk);
            // Wait for boot (max 90s)
            for _ in 0..45 {
                if let Ok(output) = std::process::Command::new(&adb)
                    .args(["-s", &serial, "shell", "getprop", "sys.boot_completed"])
                    .output()
                {
                    if String::from_utf8_lossy(&output.stdout).trim() == "1" {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            let _ = fingerprint::apply_to_device(&sdk, &serial, &fp);
        });
    }

    Ok(port)
}

#[tauri::command]
fn stop_device(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    emu_store: tauri::State<Arc<EmulatorStore>>,
    id: String,
) -> Result<(), String> {
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let dev = store.get(&id).ok_or("Device not found")?;
    emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
    store.update_status(&id, "stopped");
    Ok(())
}

#[tauri::command]
fn check_device_alive(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<bool, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    Ok(EmulatorStore::is_alive(&sdk_path, dev.port))
}

// ── ADB ──
#[tauri::command]
fn adb_shell(config: tauri::State<Arc<Mutex<Config>>>, store: tauri::State<Arc<DeviceStore>>, id: String, cmd: String) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::shell(&sdk_path, &serial, &cmd)
}

#[tauri::command]
fn install_apk(config: tauri::State<Arc<Mutex<Config>>>, store: tauri::State<Arc<DeviceStore>>, id: String, apk_path: String) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::install_apk(&sdk_path, &serial, &apk_path)
}

// ── Proxy ──
#[tauri::command]
fn set_device_proxy(
    config: tauri::State<Arc<Mutex<Config>>>,
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
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);

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
    fingerprint::list_profiles(&paths::profiles_dir())
}

#[tauri::command]
fn create_profile(profile: FingerprintProfile) -> Result<FingerprintProfile, String> {
    fingerprint::save_profile(&paths::profiles_dir(), &profile);
    Ok(profile)
}

#[tauri::command]
fn delete_profile(name: String) -> Result<(), String> {
    fingerprint::delete_profile(&paths::profiles_dir(), &name)
}

#[tauri::command]
fn apply_profile(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    device_id: String,
    profile_name: String,
) -> Result<(), String> {
    let dev = store.get(&device_id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to apply profile".into());
    }
    let profiles = fingerprint::list_profiles(&paths::profiles_dir());
    let profile = profiles.into_iter().find(|p| p.name == profile_name).ok_or("Profile not found")?;
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    fingerprint::apply_to_device(&sdk_path, &serial, &profile)
}

#[tauri::command]
fn set_device_identity(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    device_id: String,
    imei: Option<String>,
    imei2: Option<String>,
    meid: Option<String>,
    phone_number: Option<String>,
    sim_operator: Option<String>,
    sim_operator_name: Option<String>,
    sim_country: Option<String>,
    sim_serial: Option<String>,
) -> Result<(), String> {
    let dev = store.get(&device_id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);

    // Restart adbd as root so setprop works for persist.radio.* / gsm.*
    let _ = adb_bridge::shell(&sdk_path, &serial, "root");

    if let Some(v) = &imei { adb_bridge::setprop(&sdk_path, &serial, "persist.radio.imei", v)?; }
    if let Some(v) = &imei2 { adb_bridge::setprop(&sdk_path, &serial, "persist.radio.imei2", v)?; }
    if let Some(v) = &meid { adb_bridge::setprop(&sdk_path, &serial, "persist.radio.meid", v)?; }
    if let Some(v) = &phone_number { adb_bridge::setprop(&sdk_path, &serial, "gsm.sim.phone_number", v)?; }
    if let Some(v) = &sim_operator {
        adb_bridge::setprop(&sdk_path, &serial, "gsm.sim.operator.numeric", v)?;
        adb_bridge::setprop(&sdk_path, &serial, "gsm.operator.numeric", v)?;
    }
    if let Some(v) = &sim_operator_name {
        adb_bridge::setprop(&sdk_path, &serial, "gsm.sim.operator.alpha", v)?;
        adb_bridge::setprop(&sdk_path, &serial, "gsm.operator.alpha", v)?;
    }
    if let Some(v) = &sim_country {
        adb_bridge::setprop(&sdk_path, &serial, "gsm.sim.operator.iso-country", v)?;
        adb_bridge::setprop(&sdk_path, &serial, "gsm.operator.iso-country", v)?;
    }
    if let Some(v) = &sim_serial { adb_bridge::setprop(&sdk_path, &serial, "gsm.sim.serial", v)?; }

    Ok(())
}

// ── Extras: Recording, Clipboard, GPS, Logcat ──
#[tauri::command]
fn start_screen_record(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    rec_store: tauri::State<Arc<RecordingStore>>,
    id: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::start_recording(&sdk_path, &serial, &id, &rec_store)
}

#[tauri::command]
fn stop_screen_record(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    rec_store: tauri::State<Arc<RecordingStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let local_dir = std::env::temp_dir().join("enmulator_recordings");
    extras::stop_recording(&sdk_path, &serial, &id, &rec_store, &local_dir)
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn clipboard_sync(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    direction: String,
    text: Option<String>,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::sync_clipboard(&sdk_path, &serial, &direction, text.as_deref())
}

#[tauri::command]
fn gps_set(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    lat: f64,
    lon: f64,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    extras::set_gps(&sdk_path, &serial, lat, lon)
}

#[tauri::command]
fn logcat_start(
    app: tauri::AppHandle,
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
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
fn list_files(config: tauri::State<Arc<Mutex<Config>>>, store: tauri::State<Arc<DeviceStore>>, id: String, path: String) -> Result<Vec<FileEntry>, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let quoted_path = path.replace('\'', "'\\''");
    let output = adb_bridge::shell(&sdk_path, &serial, &format!("ls -la '{}'", quoted_path))?;
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
        // Reconstruct name from columns 8+ (may contain spaces)
        let name_raw = parts[8..].join(" ");
        // Strip symlink target: "name -> /path/to/target" → "name"
        let name = match name_raw.find(" -> ") {
            Some(idx) => name_raw[..idx].to_string(),
            None => name_raw,
        };
        entries.push(FileEntry { name, is_dir, size, permissions: perms });
    }
    Ok(entries)
}

#[tauri::command]
fn pull_file(config: tauri::State<Arc<Mutex<Config>>>, store: tauri::State<Arc<DeviceStore>>, id: String, remote_path: String, local_path: String) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::pull(&sdk_path, &serial, &remote_path, &local_path)
}

#[tauri::command]
fn push_file(config: tauri::State<Arc<Mutex<Config>>>, store: tauri::State<Arc<DeviceStore>>, id: String, local_path: String, remote_path: String) -> Result<(), String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    let serial = format!("emulator-{}", dev.port);
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    adb_bridge::push(&sdk_path, &serial, &local_path, &remote_path)
}

// ── Host File Explorer ──

#[tauri::command]
fn list_host_files(path: String) -> Result<Vec<FileEntry>, String> {
    let dir_path = std::path::Path::new(&path);
    if !dir_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    if !dir_path.is_dir() {
        return Err(format!("Not a directory: {}", path));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let permissions = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = meta.permissions().mode() & 0o777;
                format!("{}{}{}{}{}{}{}{}{}{}",
                    if meta.is_dir() { 'd' } else { '-' },
                    if mode & 0o400 != 0 { 'r' } else { '-' },
                    if mode & 0o200 != 0 { 'w' } else { '-' },
                    if mode & 0o100 != 0 { 'x' } else { '-' },
                    if mode & 0o040 != 0 { 'r' } else { '-' },
                    if mode & 0o020 != 0 { 'w' } else { '-' },
                    if mode & 0o010 != 0 { 'x' } else { '-' },
                    if mode & 0o004 != 0 { 'r' } else { '-' },
                    if mode & 0o002 != 0 { 'w' } else { '-' },
                    if mode & 0o001 != 0 { 'x' } else { '-' },
                )
            }
            #[cfg(not(unix))]
            {
                if meta.is_dir() { "drwxr-xr-x".to_string() } else { "-rw-r--r--".to_string() }
            }
        };
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            permissions,
        });
    }
    // Directories first, then alphabetical
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    Ok(entries)
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
    config: tauri::State<'_, Arc<Mutex<Config>>>,
    store: tauri::State<'_, Arc<DeviceStore>>,
    emu_store: tauri::State<'_, Arc<EmulatorStore>>,
    ids: Vec<String>,
) -> Result<BatchResult, String> {
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
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
        let profile = dev.fingerprint_profile.as_ref().and_then(|name| {
            fingerprint::list_profiles(&paths::profiles_dir())
                .into_iter().find(|p| &p.name == name)
        });
        match emu_store.start(&sdk_path, &dev.avd_name, false, profile.as_ref()) {
            Ok(port) => {
                store.update_port(id, port);
                store.update_status(id, "running");
                // Apply identity after boot (fire-and-forget)
                if let Some(fp) = profile {
                    let sdk = sdk_path.clone();
                    let serial = format!("emulator-{}", port);
                    std::thread::spawn(move || {
                        let adb = sdk::get_adb_path(&sdk);
                        for _ in 0..45 {
                            if let Ok(o) = std::process::Command::new(&adb)
                                .args(["-s", &serial, "shell", "getprop", "sys.boot_completed"])
                                .output()
                            {
                                if String::from_utf8_lossy(&o.stdout).trim() == "1" { break; }
                            }
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        }
                        let _ = fingerprint::apply_to_device(&sdk, &serial, &fp);
                    });
                }
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
    config: tauri::State<'_, Arc<Mutex<Config>>>,
    store: tauri::State<'_, Arc<DeviceStore>>,
    emu_store: tauri::State<'_, Arc<EmulatorStore>>,
    ids: Vec<String>,
) -> Result<BatchResult, String> {
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
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
    config: tauri::State<'_, Arc<Mutex<Config>>>,
    store: tauri::State<'_, Arc<DeviceStore>>,
    emu_store: tauri::State<'_, Arc<EmulatorStore>>,
    ids: Vec<String>,
) -> Result<BatchResult, String> {
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let mut result = BatchResult { success: vec![], failed: vec![] };

    for id in &ids {
        if let Some(dev) = store.get(id) {
            if dev.status == "running" {
                emu_store.stop(&sdk_path, &dev.avd_name, dev.port);
            }
            let _ = avd_manager::delete_avd(&sdk_path, &dev.avd_name);
        }
        store.remove(id);
        result.success.push(id.clone());
    }
    Ok(result)
}

// ── Device Extras: Android ID, WiFi MAC, Timezone, Locale ──

#[tauri::command]
fn set_device_extras(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    android_id: Option<String>,
    wifi_mac: Option<String>,
    timezone: Option<String>,
    locale: Option<String>,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );
    let serial = format!("emulator-{}", dev.port);
    let mut applied = vec![];

    if let Some(ref aid) = android_id {
        adb_bridge::shell(&sdk_path, &serial,
            &format!("settings put secure android_id {}", aid))?;
        applied.push(format!("Android ID → {}", aid));
    }

    if let Some(ref mac) = wifi_mac {
        // Try wlan0 first, fall back to eth0 (emulator uses eth0 for WiFi simulation)
        let mac_cmd = format!(
            "ip link set wlan0 address {mac} 2>/dev/null || ip link set eth0 address {mac}",
            mac = mac
        );
        adb_bridge::shell(&sdk_path, &serial, &mac_cmd)?;
        applied.push(format!("WiFi MAC → {}", mac));
    }

    if let Some(ref tz) = timezone {
        adb_bridge::shell(&sdk_path, &serial,
            &format!("settings put global time_zone '{}'", tz))?;
        let _ = adb_bridge::setprop(&sdk_path, &serial, "persist.sys.timezone", tz);
        applied.push(format!("Timezone → {}", tz));
    }

    if let Some(ref loc) = locale {
        // system_locales for API 24+; system locale for older
        let _ = adb_bridge::shell(&sdk_path, &serial,
            &format!("settings put system system_locales '{}'", loc));
        let _ = adb_bridge::shell(&sdk_path, &serial,
            &format!("settings put system locale '{}'", loc));
        // Broadcast locale change so running apps pick it up
        let _ = adb_bridge::shell(&sdk_path, &serial,
            "am broadcast -a android.intent.action.LOCALE_CHANGED");
        applied.push(format!("Locale → {}", loc));
    }

    if applied.is_empty() {
        return Err("No values provided".into());
    }
    Ok(applied.join("\n"))
}

// ── Root ──

#[tauri::command]
fn toggle_root(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to toggle root".into());
    }
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    
    if dev.root_enabled {
        let adb = sdk::get_adb_path(&sdk_path);
        let out = std::process::Command::new(&adb)
            .args(["-s", &serial, "unroot"])
            .output()
            .map_err(|e| format!("ADB error: {}", e))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        if out.status.success() || stdout.contains("not running as root") || stdout.contains("restarting") {
            store.set_root(&id, false);
            Ok("Root disabled".to_string())
        } else {
            Err(format!("adb unroot failed: {}", stdout.trim()))
        }
    } else {
        let adb = sdk::get_adb_path(&sdk_path);
        let output = std::process::Command::new(&adb)
            .args(["-s", &serial, "root"])
            .output()
            .map_err(|e| format!("ADB error: {}", e))?;
        
        let out = String::from_utf8_lossy(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", out, err).to_lowercase();

        if out.contains("already running as root") || output.status.success() {
            store.set_root(&id, true);
            Ok("Root enabled (adb root — ADB daemon runs as root)".to_string())
        } else if combined.contains("cannot run as root") || combined.contains("production build") || combined.contains("not allowed") {
            Err(
                "adb root is not supported on this system image.\n\n\
                ✓ Use a 'google_apis' (non-Play Store) system image for adb root.\n\
                ✓ For Play Store images, use 'Magisk Root' instead (device must be stopped first)."
                .to_string()
            )
        } else {
            Err(format!(
                "adb root failed.\n\nOutput: {}\n\nTip: Use a 'google_apis' (non-Play Store) system image.",
                combined.trim()
            ))
        }
    }
}

// ── System Disk Mode ──

#[tauri::command]
fn toggle_writable_system(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to change system disk mode".into());
    }
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );
    let serial = format!("emulator-{}", dev.port);
    let adb = sdk::get_adb_path(&sdk_path);

    if dev.writable_system {
        // Switch back to read-only
        let _ = std::process::Command::new(&adb)
            .args(["-s", &serial, "shell", "mount", "-o", "remount,ro", "/system"])
            .output();
        store.set_writable_system(&id, false);
        Ok("System disk set to read-only".to_string())
    } else {
        // Ensure adb is running as root first
        let root_out = std::process::Command::new(&adb)
            .args(["-s", &serial, "root"])
            .output()
            .map_err(|e| format!("adb root error: {}", e))?;

        let root_stdout = String::from_utf8_lossy(&root_out.stdout);
        if !root_out.status.success()
            && !root_stdout.contains("already running as root")
            && !root_stdout.contains("restarting adbd as root")
        {
            return Err(
                "Root required for writable system. Use a google_apis (non-Play Store) system image and enable root first.".to_string()
            );
        }

        // Wait briefly for adbd to restart as root
        std::thread::sleep(std::time::Duration::from_millis(800));

        let remount_out = std::process::Command::new(&adb)
            .args(["-s", &serial, "remount"])
            .output()
            .map_err(|e| format!("adb remount error: {}", e))?;

        if !remount_out.status.success() {
            let stderr = String::from_utf8_lossy(&remount_out.stderr);
            return Err(format!("adb remount failed: {}", stderr));
        }
        store.set_writable_system(&id, true);
        store.set_root(&id, true);
        Ok("System disk set to writable (changes are runtime only — reboot resets)".to_string())
    }
}

// ── rootAVD (Magisk) integration ──

#[tauri::command]
fn root_with_magisk(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status == "running" {
        return Err("Stop the device before rooting with Magisk".into());
    }
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );

    // Locate the ramdisk for this device's system image
    // avd_name format: enmulator_{id}
    let avd_dir = store.devices_dir.join(&dev.id);
    let config_ini = avd_dir.join("config.ini");

    // Read the system image path from config.ini
    let ini_content = std::fs::read_to_string(&config_ini)
        .map_err(|e| format!("Cannot read config.ini: {}", e))?;

    // config.ini uses either "key=value" or "key = value" format
    let image_sysdir = ini_content.lines()
        .find(|l| {
            let t = l.trim();
            t.starts_with("image.sysdir.1")
                && t[14..].trim_start().starts_with('=')
        })
        .and_then(|l| l.splitn(2, '=').nth(1))
        .map(|v| v.trim().to_string())
        .ok_or_else(|| format!(
            "Cannot find image.sysdir.1 in config.ini.\nPath: {}\nContent preview: {}",
            config_ini.display(),
            ini_content.lines().take(10).collect::<Vec<_>>().join(" | ")
        ))?;

    let ramdisk = sdk_path.join(&image_sysdir).join("ramdisk.img");
    if !ramdisk.exists() {
        return Err(format!("ramdisk.img not found at: {}", ramdisk.display()));
    }

    // Locate rootAVD script — platform-specific file
    #[cfg(target_os = "windows")]
    let script_name = "rootAVD.bat";
    #[cfg(not(target_os = "windows"))]
    let script_name = "rootAVD.sh";

    let res_dir = paths::resource_dir();
    let exe_dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from("../src-tauri/rootAVD").join(script_name), // dev (project root)
        PathBuf::from("src-tauri/rootAVD").join(script_name),    // dev alt
        exe_dir.join("rootAVD").join(script_name),
    ];
    if let Some(ref rd) = res_dir {
        candidates.push(rd.join("rootAVD").join(script_name));
    }

    let rootavd = candidates.iter().find(|p| p.exists())
        .ok_or_else(|| format!(
            "{} not found. Candidates checked:\n{}",
            script_name,
            candidates.iter().map(|p| format!("  {}", p.display())).collect::<Vec<_>>().join("\n")
        ))?;

    // Canonicalize to absolute path — relative paths break when current_dir is set
    let rootavd_abs = std::fs::canonicalize(rootavd)
        .unwrap_or_else(|_| rootavd.clone());
    let rootavd_dir = rootavd_abs.parent().unwrap_or(std::path::Path::new("."));
    let ramdisk_str = ramdisk.to_str().unwrap_or_default();

    // rootAVD directly patches ramdisk.img in place with Magisk.
    // The system image is shared — rooting once affects all AVDs using the same image.
    #[cfg(target_os = "windows")]
    let output = std::process::Command::new("cmd")
        .args(["/c", rootavd_abs.to_str().unwrap_or_default()])
        .arg(ramdisk_str)
        .current_dir(rootavd_dir)
        .output()
        .map_err(|e| format!("rootAVD execution error: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = std::process::Command::new("bash")
        .arg(&rootavd_abs)
        .arg(ramdisk_str)
        .current_dir(rootavd_dir)
        .output()
        .map_err(|e| format!("rootAVD execution error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() || stdout.contains("Magisk") || stdout.contains("patched") {
        Ok(format!(
            "Magisk patched into ramdisk.img ✓\n\n\
            Next steps:\n\
            1. Start this device\n\
            2. Click 'Install Magisk App' in Quick Actions to install the Magisk Manager\n\
            3. Root is already active — apps can request su\n\n\
            ⚠️  This system image is shared. All devices using the same image are affected.\n\n\
            Image: {}",
            ramdisk.display()
        ))
    } else {
        Err(format!(
            "rootAVD failed (exit: {:?})\n\nstdout:\n{}\nstderr:\n{}",
            output.status.code(), stdout, stderr
        ))
    }
}

// ── Install Magisk Manager APK ──

#[tauri::command]
fn install_magisk_apk(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running to install Magisk app".into());
    }
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );

    // Find bundled Magisk.zip
    let res_dir = paths::resource_dir();
    let exe_dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    let zip_candidates = [
        PathBuf::from("../src-tauri/rootAVD/Magisk.zip"),
        PathBuf::from("src-tauri/rootAVD/Magisk.zip"),
        exe_dir.join("rootAVD/Magisk.zip"),
    ];
    let mut zip_candidates = zip_candidates.to_vec();
    if let Some(ref rd) = res_dir {
        zip_candidates.push(rd.join("rootAVD/Magisk.zip"));
    }
    let magisk_zip = zip_candidates.iter().find(|p| p.exists())
        .ok_or("Magisk.zip not found")?;

    // Magisk.zip is itself an APK (APKs are ZIP files) — install it directly
    let tmp_apk = std::env::temp_dir().join("Magisk.apk");
    std::fs::copy(magisk_zip, &tmp_apk).map_err(|e| format!("Failed to copy Magisk: {}", e))?;

    let serial = format!("emulator-{}", dev.port);
    let result = adb_bridge::install_apk(&sdk_path, &serial, &tmp_apk.to_string_lossy())?;
    let _ = std::fs::remove_file(&tmp_apk);

    Ok(format!("Magisk Manager installed ✓\n{}", result.trim()))
}

// ── Device Templates ──
#[tauri::command]
fn list_device_templates(config: tauri::State<Arc<Mutex<Config>>>) -> Result<Vec<String>, String> {
    let sdk_path = PathBuf::from(
        config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?
    );
    sdk::check_sdk_tools(&sdk_path)?;
    avd_manager::list_device_definitions(&sdk_path)
}

// ── Config ──
#[tauri::command]
fn get_config(config: tauri::State<Arc<Mutex<Config>>>) -> Config {
    config.lock().unwrap().clone()
}

#[tauri::command]
fn set_sdk_path(config: tauri::State<Arc<Mutex<Config>>>, config_path_state: tauri::State<PathBuf>, path: String) -> Result<(), String> {
    let mut cfg = config.lock().unwrap();
    cfg.sdk_path = Some(path);
    config::save(&config_path_state, &cfg);
    Ok(())
}

#[tauri::command]
fn update_config(
    config: tauri::State<Arc<Mutex<Config>>>,
    config_path: tauri::State<PathBuf>,
    sdk_path: Option<String>,
    devices_dir: Option<String>,
    api_server_port: Option<u16>,
    default_headless: Option<bool>,
    auto_start_api: Option<bool>,
    default_api_level: Option<u8>,
    default_abi: Option<String>,
    default_tag: Option<String>,
) -> Result<(), String> {
    let mut cfg = config.lock().unwrap();
    if let Some(v) = sdk_path { cfg.sdk_path = Some(v); }
    if let Some(v) = devices_dir { cfg.devices_dir = v; }
    if let Some(v) = api_server_port { cfg.api_server_port = v; }
    if let Some(v) = default_headless { cfg.default_headless = v; }
    if let Some(v) = auto_start_api { cfg.auto_start_api = v; }
    if let Some(v) = default_api_level { cfg.default_api_level = v; }
    if let Some(v) = default_abi { cfg.default_abi = v; }
    if let Some(v) = default_tag { cfg.default_tag = v; }
    config::save(&config_path, &cfg);
    Ok(())
}

#[tauri::command]
fn bypass_detection(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
    let serial = format!("emulator-{}", dev.port);
    bypass::bypass_detection(&sdk_path, &serial)
}

// ── Cert Installer ──

#[tauri::command]
fn install_cert(
    config: tauri::State<Arc<Mutex<Config>>>,
    store: tauri::State<Arc<DeviceStore>>,
    id: String,
    cert_path: String,
) -> Result<String, String> {
    let dev = store.get(&id).ok_or("Device not found")?;
    if dev.status != "running" {
        return Err("Device must be running".into());
    }
    let sdk_path = PathBuf::from(config.lock().unwrap().sdk_path.as_ref().ok_or("SDK not configured")?);
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

    // 4. Compute the subject hash using the device's openssl binary.
    // Android requires system CA certs to be named <subject_hash_old>.0
    let hash_cmd = format!(
        "openssl x509 -subject_hash_old -noout -in /sdcard/{} 2>/dev/null",
        cert_file
    );
    let hash_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", &hash_cmd])
        .output()
        .map_err(|e| format!("hash compute error: {}", e))?;
    let hash = String::from_utf8_lossy(&hash_out.stdout).trim().to_string();

    // Validate: openssl subject_hash_old produces an 8-char hex string
    let dest_name = if hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("{}.0", hash)
    } else {
        // openssl not available on device — fall back to original name with a warning
        eprintln!("Warning: could not compute cert hash (openssl unavailable?). Using original filename — Android may not trust the cert.");
        cert_file.clone()
    };

    // 5. Copy to /system/etc/security/cacerts/<hash>.0
    let cp_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "cp",
               &format!("/sdcard/{}", cert_file),
               &format!("/system/etc/security/cacerts/{}", dest_name)])
        .output()
        .map_err(|e| format!("adb shell cp error: {}", e))?;
    if !cp_out.status.success() {
        return Err(format!("copy cert failed: {}", String::from_utf8_lossy(&cp_out.stderr)));
    }

    // 6. Set permissions 644
    let chmod_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "chmod", "644",
               &format!("/system/etc/security/cacerts/{}", dest_name)])
        .output()
        .map_err(|e| format!("adb shell chmod error: {}", e))?;
    if !chmod_out.status.success() {
        return Err(format!("chmod failed: {}", String::from_utf8_lossy(&chmod_out.stderr)));
    }

    // 7. Remount /system ro
    let ro_out = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "mount", "-o", "remount,ro", "/system"])
        .output()
        .map_err(|e| format!("adb shell mount ro error: {}", e))?;
    if !ro_out.status.success() {
        return Err(format!("remount ro failed: {}", String::from_utf8_lossy(&ro_out.stderr)));
    }

    // 8. Clean up /sdcard/ copy
    let _ = std::process::Command::new(&adb)
        .args(["-s", &serial, "shell", "rm", &format!("/sdcard/{}", cert_file)])
        .output();

    Ok(format!("Certificate installed as {} in /system/etc/security/cacerts/", dest_name))
}

fn main() {
    // Copy default profiles on first run
    paths::ensure_default_profiles();

    let config_path = paths::config_file();
    let mut cfg = config::load(&config_path);

    // Auto-detect SDK on first run if not already configured
    if cfg.sdk_path.is_none() {
        if let Some(detected) = sdk::detect_sdk() {
            cfg.sdk_path = Some(detected.to_string_lossy().to_string());
            config::save(&config_path, &cfg);
        }
    }

    // Use the configured devices_dir, or fall back to platform data dir
    let devices_dir = if cfg.devices_dir != Config::default().devices_dir {
        PathBuf::from(&cfg.devices_dir)
    } else {
        paths::devices_dir()
    };

    let store = Arc::new(DeviceStore::new(devices_dir));
    let existing = store.list();
    let emu_store = Arc::new(EmulatorStore::new(&existing));
    let rec_store = Arc::new(RecordingStore::new());
    let proxy_store = Arc::new(ProxyStore::new());

    // Shared mutable config — same Arc used by both the Tauri commands and the REST API
    let shared_config: Arc<Mutex<Config>> = Arc::new(Mutex::new(cfg));

    let api_state = Arc::new(api_server::AppState {
        device_store: store.clone(),
        emulator_store: emu_store.clone(),
        config: shared_config.clone(),
    });

    // Auto-start REST API server if configured
    {
        let c = shared_config.lock().unwrap();
        if c.auto_start_api {
            let port = c.api_server_port;
            drop(c);
            let _ = api_server::start_api_server(api_state.clone(), port);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(shared_config)
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
            set_device_identity,
            start_screen_record,
            stop_screen_record,
            clipboard_sync,
            gps_set,
            logcat_start,
            start_api_server,
            stop_api_server,
            get_config,
            set_sdk_path,
            update_config,
            list_device_templates,
            list_files,
            pull_file,
            push_file,
            set_device_extras,
            toggle_root,
            toggle_writable_system,
            root_with_magisk,
            install_magisk_apk,
            bypass_detection,
            install_cert,
            list_host_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running enmulator");
}
