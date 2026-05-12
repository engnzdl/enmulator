use std::path::PathBuf;

/// Base config directory for the app: ~/.config/enmulator (Linux),
/// ~/Library/Application Support/enmulator (macOS), %APPDATA%/enmulator (Windows)
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("enmulator")
}

/// Base data directory: ~/.local/share/enmulator (Linux),
/// ~/Library/Application Support/enmulator (macOS), %LOCALAPPDATA%/enmulator (Windows)
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("enmulator")
}

/// Config file path
pub fn config_file() -> PathBuf {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

/// Devices directory (where AVD data is stored)
pub fn devices_dir() -> PathBuf {
    let dir = data_dir().join("devices");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Profiles directory — bundled defaults live here, users can add custom ones
pub fn profiles_dir() -> PathBuf {
    let dir = config_dir().join("profiles");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Copy default profiles from the app's resource bundle to config dir on first run
pub fn ensure_default_profiles() {
    let dest = profiles_dir();
    // Check if profiles already exist
    if dest.join("samsung_s23_ultra.json").exists() {
        return;
    }

    // Try resource dir first (production bundle), fall back to relative path (dev)
    let src = if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // Production: profiles are in Resources/profiles (macOS bundle) or alongside exe
            let candidates = [
                parent.join("profiles"),
                parent.join("../profiles"),
                parent.join("Resources/profiles"),
            ];
            let mut found = None;
            for c in &candidates {
                if c.join("samsung_s23_ultra.json").exists() {
                    found = Some(c.clone());
                    break;
                }
            }
            found
        } else {
            None
        }
    } else {
        None
    };

    // Dev fallback: relative to CWD (src-tauri/../profiles)
    let src = src.unwrap_or_else(|| PathBuf::from("../profiles"));

    // Copy all .json profile files
    if src.exists() {
        if let Ok(entries) = std::fs::read_dir(&src) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Some(name) = path.file_name() {
                        let _ = std::fs::copy(&path, dest.join(name));
                    }
                }
            }
        }
    }
}
