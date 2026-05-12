use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize)]
pub struct SystemImage {
    pub api_level: u8,
    pub abi: String,
    pub tag: String,
    pub description: String,
}

/// Platform-aware binary name helpers.
/// On Windows, Android SDK tools use .exe or .bat extensions.
#[cfg(target_os = "windows")]
fn adb_bin() -> &'static str { "adb.exe" }
#[cfg(not(target_os = "windows"))]
fn adb_bin() -> &'static str { "adb" }

#[cfg(target_os = "windows")]
fn emulator_bin() -> &'static str { "emulator.exe" }
#[cfg(not(target_os = "windows"))]
fn emulator_bin() -> &'static str { "emulator" }

#[cfg(target_os = "windows")]
fn avdmanager_bin() -> &'static str { "avdmanager.bat" }
#[cfg(not(target_os = "windows"))]
fn avdmanager_bin() -> &'static str { "avdmanager" }

/// Wraps SDK tool invocations correctly per platform.
/// On Windows, .bat files need `cmd /c` prefix when paths may contain spaces.
#[cfg(target_os = "windows")]
pub fn sdk_command(path: &PathBuf) -> Command {
    if path.to_string_lossy().ends_with(".bat") {
        let mut c = Command::new("cmd");
        c.arg("/c").arg(path);
        c
    } else {
        Command::new(path)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn sdk_command(path: &PathBuf) -> Command {
    Command::new(path)
}

#[cfg(target_os = "windows")]
fn sdkmanager_bin() -> &'static str { "sdkmanager.bat" }
#[cfg(not(target_os = "windows"))]
fn sdkmanager_bin() -> &'static str { "sdkmanager" }

pub fn detect_sdk() -> Option<PathBuf> {
    for var in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(path) = std::env::var(var) {
            let p = PathBuf::from(path);
            if p.join("platform-tools").join(adb_bin()).exists() {
                return Some(p);
            }
        }
    }
    let home = dirs::home_dir()?;
    let candidates = [
        #[cfg(target_os = "macos")] home.join("Library/Android/sdk"),
        #[cfg(target_os = "linux")] home.join("Android/Sdk"),
        #[cfg(target_os = "windows")] home.join("AppData/Local/Android/Sdk"),
    ];
    for path in &candidates {
        if path.join("platform-tools").join(adb_bin()).exists() {
            return Some(path.clone());
        }
    }
    None
}

pub fn get_avdmanager_path(sdk: &PathBuf) -> PathBuf {
    let new_path = sdk.join("cmdline-tools/latest/bin").join(avdmanager_bin());
    if new_path.exists() { return new_path; }
    sdk.join("tools/bin").join(avdmanager_bin())
}

pub fn get_emulator_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("emulator").join(emulator_bin())
}

pub fn get_adb_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("platform-tools").join(adb_bin())
}

pub fn get_sdkmanager_path(sdk: &PathBuf) -> PathBuf {
    let new_path = sdk.join("cmdline-tools/latest/bin").join(sdkmanager_bin());
    if new_path.exists() { return new_path; }
    sdk.join("tools/bin").join(sdkmanager_bin())
}

/// Returns error if cmdline-tools are not installed
pub fn check_sdk_tools(sdk: &PathBuf) -> Result<(), String> {
    let sm = get_sdkmanager_path(sdk);
    let avd = get_avdmanager_path(sdk);
    if !sm.exists() || !avd.exists() {
        return Err(format!(
            "Android SDK Command-line Tools not found.\n\n\
            Install them via Android Studio:\n\
            SDK Manager → SDK Tools → \"Android SDK Command-line Tools\"\n\n\
            Or download from:\n\
            https://developer.android.com/studio#command-line-tools-only\n\n\
            Expected at: {}/cmdline-tools/latest/bin/",
            sdk.to_string_lossy()
        ));
    }
    Ok(())
}

pub fn get_ramdisk_path(sdk: &PathBuf, api_level: u8, tag: &str, abi: &str) -> PathBuf {
    sdk.join("system-images")
        .join(format!("android-{}", api_level))
        .join(tag)
        .join(abi)
        .join("ramdisk.img")
}

/// Calls `sdkmanager --list` and parses available system images.
/// Returns Vec<SystemImage> with api_level, abi, tag, and description.
pub fn list_available_images(sdk_path: &PathBuf) -> Result<Vec<SystemImage>, String> {
    let sdkmanager = get_sdkmanager_path(sdk_path);
    let output = sdk_command(&sdkmanager)
        .args(["--list"])
        .output()
        .map_err(|e| format!("sdkmanager error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut images = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        // Lines look like:
        //   system-images;android-34;google_apis;x86_64    | 14 | Google APIs Intel x86_64 Atom System Image
        if !trimmed.starts_with("system-images;android-") {
            continue;
        }
        // Split on the first '|' to separate package path from description
        let parts: Vec<&str> = trimmed.splitn(2, '|').collect();
        if parts.is_empty() {
            continue;
        }
        let package_path = parts[0].trim();
        let description = if parts.len() > 1 {
            parts[1].trim().to_string()
        } else {
            String::new()
        };

        // Parse: system-images;android-{api};{tag};{abi}
        let segments: Vec<&str> = package_path.split(';').collect();
        if segments.len() < 4 {
            continue;
        }
        // segments[0] = "system-images"
        // segments[1] = "android-34"
        // segments[2] = "google_apis" (tag)
        // segments[3] = "x86_64" (abi)
        let api_str = segments[1].strip_prefix("android-").unwrap_or(segments[1]);
        let api_level: u8 = match api_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tag = segments[2].to_string();
        let abi = segments[3].to_string();

        images.push(SystemImage {
            api_level,
            abi,
            tag,
            description,
        });
    }

    // Sort by API level descending, then ABI, then tag
    images.sort_by(|a, b| {
        b.api_level
            .cmp(&a.api_level)
            .then_with(|| a.abi.cmp(&b.abi))
            .then_with(|| a.tag.cmp(&b.tag))
    });
    images.dedup_by(|a, b| a.api_level == b.api_level && a.abi == b.abi && a.tag == b.tag);

    Ok(images)
}

/// Installs a system image package via sdkmanager.
/// The package should be in the format: "system-images;android-{api};{tag};{abi}"
/// Streams stdout lines in real-time via Tauri `download-progress` events.
pub fn install_system_image(app: &tauri::AppHandle, sdk_path: &PathBuf, package: &str) -> Result<(), String> {
    let sdkmanager = get_sdkmanager_path(sdk_path);

    let mut child = sdk_command(&sdkmanager)
        .args(["--install", package])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("sdkmanager install spawn error: {}", e))?;

    // Auto-accept license prompts by writing 'y' to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"y\ny\ny\ny\ny\ny\ny\ny\ny\ny\n");
    }

    let stdout = child.stdout.take().ok_or("Failed to capture sdkmanager stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture sdkmanager stderr")?;

    let app_handle = app.clone();
    let pkg = package.to_string();

    // Stream stdout lines as progress events in a separate thread
    let stdout_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    let payload = serde_json::json!({
                        "package": pkg,
                        "line": text,
                    });
                    let _ = app_handle.emit("download-progress", payload);
                }
                Err(_) => break,
            }
        }
    });

    // Collect stderr for error reporting
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut buf = String::new();
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    buf.push_str(&text);
                    buf.push('\n');
                }
                Err(_) => break,
            }
        }
        buf
    });

    // Wait for stdout reading to finish
    stdout_handle.join().map_err(|_| "stdout thread panicked".to_string())?;

    // Wait for process to exit
    let status = child.wait().map_err(|e| format!("sdkmanager wait error: {}", e))?;
    let stderr_text = stderr_handle.join().map_err(|_| "stderr thread panicked".to_string())?;

    if !status.success() {
        if stderr_text.to_lowercase().contains("already installed") {
            return Ok(());
        }
        return Err(stderr_text);
    }
    Ok(())
}
