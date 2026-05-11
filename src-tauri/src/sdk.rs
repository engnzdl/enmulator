use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct SystemImage {
    pub api_level: u8,
    pub abi: String,
    pub tag: String,
    pub description: String,
}

pub fn detect_sdk() -> Option<PathBuf> {
    for var in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(path) = std::env::var(var) {
            let p = PathBuf::from(path);
            if p.join("platform-tools/adb").exists() {
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
        if path.join("platform-tools/adb").exists() {
            return Some(path.clone());
        }
    }
    None
}

pub fn get_avdmanager_path(sdk: &PathBuf) -> PathBuf {
    // Try new cmdline-tools first, then old tools/bin
    let new_path = sdk.join("cmdline-tools/latest/bin/avdmanager");
    if new_path.exists() { return new_path; }
    sdk.join("tools/bin/avdmanager")
}

pub fn get_emulator_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("emulator/emulator")
}

pub fn get_adb_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("platform-tools/adb")
}

pub fn get_sdkmanager_path(sdk: &PathBuf) -> PathBuf {
    let new_path = sdk.join("cmdline-tools/latest/bin/sdkmanager");
    if new_path.exists() { return new_path; }
    sdk.join("tools/bin/sdkmanager")
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
    let output = Command::new(&sdkmanager)
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
pub fn install_system_image(sdk_path: &PathBuf, package: &str) -> Result<(), String> {
    let sdkmanager = get_sdkmanager_path(sdk_path);
    
    // First accept all licenses
    let _ = Command::new(&sdkmanager)
        .args(["--licenses"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            // Write 'y' repeatedly to accept all licenses
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = writeln!(stdin, "y\ny\ny\ny\ny\ny\ny\ny\ny\ny");
            }
            child.wait()
        });
    
    let output = Command::new(&sdkmanager)
        .args(["--install", package])
        .output()
        .map_err(|e| format!("sdkmanager install error: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_lowercase().contains("already installed") {
            return Ok(());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("{}{}", stderr, stdout));
    }
    Ok(())
}
