use crate::sdk;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use tauri::Emitter;

/// Tracks in-flight screen recordings keyed by device ID.
pub struct RecordingStore {
    pub children: Mutex<HashMap<String, Child>>,
}

impl RecordingStore {
    pub fn new() -> Self {
        Self {
            children: Mutex::new(HashMap::new()),
        }
    }
}

/// Spawn `adb shell screenrecord /sdcard/recording.mp4` and save the child
/// handle so it can be killed later. Returns Ok(()) when the process starts.
pub fn start_recording(sdk_path: &PathBuf, serial: &str, device_id: &str, store: &RecordingStore) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let remote_path = format!("/sdcard/enm_recording_{}.mp4", device_id);

    let child = Command::new(&adb)
        .args(["-s", serial, "shell", "screenrecord", &remote_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start screenrecord: {}", e))?;

    store.children.lock().unwrap().insert(device_id.to_string(), child);
    Ok(())
}

/// Stop an in-progress recording: send SIGINT to the screenrecord child, wait
/// for it to finish, then pull the mp4 file to a local destination.  Returns
/// the local path the file was pulled to.
pub fn stop_recording(
    sdk_path: &PathBuf,
    serial: &str,
    device_id: &str,
    store: &RecordingStore,
    local_dir: &PathBuf,
) -> Result<PathBuf, String> {
    let remote_path = format!("/sdcard/enm_recording_{}.mp4", device_id);

    // Kill the screenrecord process on-device by sending SIGINT via adb.
    // We do this by finding the screenrecord pid and killing it, since the
    // adb shell forwarding makes it hard to signal the child directly.
    let adb = sdk::get_adb_path(sdk_path);
    Command::new(&adb)
        .args([
            "-s",
            serial,
            "shell",
            "pkill",
            "-INT",
            "-f",
            "screenrecord /sdcard/enm_recording_",
        ])
        .output()
        .map_err(|e| format!("Failed to stop screenrecord: {}", e))?;

    // Remove the local child handle (it may already be dead).
    if let Some(mut child) = store.children.lock().unwrap().remove(device_id) {
        let _ = child.wait();
    }

    // Give the device a moment to flush the mp4 file.
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Pull the recording to the local directory.
    std::fs::create_dir_all(local_dir).map_err(|e| e.to_string())?;
    let local_path = local_dir.join(format!("enm_recording_{}.mp4", device_id));

    let output = Command::new(&adb)
        .args([
            "-s",
            serial,
            "pull",
            &remote_path,
            &local_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("ADB pull error: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // Clean up the remote file.
    let _ = Command::new(&adb)
        .args(["-s", serial, "shell", "rm", "-f", &remote_path])
        .output();

    Ok(local_path)
}

/// Read or write the device clipboard.
///
/// `direction` must be `"get"` or `"set"`.  When setting, `text` is the
/// content to push to the device clipboard.
pub fn sync_clipboard(
    sdk_path: &PathBuf,
    serial: &str,
    direction: &str,
    text: Option<&str>,
) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);

    match direction {
        "get" => {
            let output = Command::new(&adb)
                .args(["-s", serial, "shell", "cmd", "clipboard", "get"])
                .output()
                .map_err(|e| format!("ADB error: {}", e))?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
            }
        }
        "set" => {
            let content = text.unwrap_or("");
            // Push clipboard text via a temp file to avoid shell escaping issues.
            let tmp_local = std::env::temp_dir().join("enm_clipboard.txt");
            std::fs::write(&tmp_local, content).map_err(|e| format!("Cannot write temp: {}", e))?;

            // Push to device, feed into clipboard, then clean up.
            let _ = Command::new(&adb)
                .args(["-s", serial, "push", &tmp_local.to_string_lossy(), "/data/local/tmp/enm_clipboard.txt"])
                .output();

            let output = Command::new(&adb)
                .args([
                    "-s",
                    serial,
                    "shell",
                    "cat /data/local/tmp/enm_clipboard.txt | cmd clipboard set",
                ])
                .output()
                .map_err(|e| format!("ADB error: {}", e))?;

            let _ = Command::new(&adb)
                .args(["-s", serial, "shell", "rm", "-f", "/data/local/tmp/enm_clipboard.txt"])
                .output();

            let _ = std::fs::remove_file(&tmp_local);

            if output.status.success() {
                Ok(String::new())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
            }
        }
        _ => Err(format!("Unknown clipboard direction: {}", direction)),
    }
}

/// Send a mock GPS location to the emulator via the console.
///
/// This uses `adb emu geo fix <longitude> <latitude>` (note the lon/lat order
/// expected by the emulator console command).
pub fn set_gps(sdk_path: &PathBuf, serial: &str, lat: f64, lon: f64) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = Command::new(&adb)
        .args([
            "-s",
            serial,
            "emu",
            "geo",
            "fix",
            &lon.to_string(),
            &lat.to_string(),
        ])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Spawn `adb logcat` in a background thread and emit each line as a Tauri
/// event named `"logcat-line"`.  The payload is `{ serial, line }`.
///
/// The returned `Child` handle can be killed to stop the stream.
pub fn stream_logcat(
    sdk_path: &PathBuf,
    serial: &str,
    app_handle: tauri::AppHandle,
) -> Result<Child, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let serial_owned = serial.to_string();

    let mut child = Command::new(&adb)
        .args(["-s", &serial_owned, "logcat"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start logcat: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to capture logcat stdout")?;

    let serial_for_thread = serial_owned.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    let payload = serde_json::json!({
                        "serial": serial_for_thread,
                        "line": text,
                    });
                    // Ignore errors — app may have closed the window.
                    let _ = app_handle.emit("logcat-line", payload);
                }
                Err(_) => break,
            }
        }
    });

    Ok(child)
}
