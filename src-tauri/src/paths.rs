use std::path::PathBuf;

/// Base config directory:
///   macOS  — ~/Library/Application Support/enmulator
///   Linux  — ~/.config/enmulator
///   Windows — %APPDATA%\enmulator
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("enmulator")
}

/// Base data directory:
///   macOS  — ~/Library/Application Support/enmulator
///   Linux  — ~/.local/share/enmulator
///   Windows — %LOCALAPPDATA%\enmulator
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("enmulator")
}

pub fn config_file() -> PathBuf {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

pub fn devices_dir() -> PathBuf {
    let dir = data_dir().join("devices");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn profiles_dir() -> PathBuf {
    let dir = config_dir().join("profiles");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Return the exe-relative bundle resource directory.
/// Layout per platform after `tauri bundle`:
///   macOS   — App.app/Contents/Resources/
///   Windows — install_dir/             (alongside exe)
///   Linux   — install_dir/             (alongside exe)
pub fn resource_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;

    // macOS: exe is in Contents/MacOS/, resources in Contents/Resources/
    let macos_resources = exe_dir.join("../Resources");
    if macos_resources.exists() {
        return Some(macos_resources.canonicalize().unwrap_or(macos_resources));
    }

    // Windows / Linux: resources are alongside the exe
    Some(exe_dir.to_path_buf())
}

/// Copy bundled default profiles to the user config dir on first run.
/// We re-copy any profile that is missing (not just on first run) so new
/// profiles added in updates are automatically available.
pub fn ensure_default_profiles() {
    let dest = profiles_dir();

    // Locate the bundled profiles directory
    let src = find_bundled_profiles();

    let src = match src {
        Some(p) => p,
        None => return, // Nothing to copy (dev environment without profiles)
    };

    if !src.exists() {
        return;
    }

    let entries = match std::fs::read_dir(&src) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Some(name) = path.file_name() {
                let target = dest.join(name);
                // Only copy if the file doesn't exist yet — preserve user edits
                if !target.exists() {
                    let _ = std::fs::copy(&path, &target);
                }
            }
        }
    }
}

fn find_bundled_profiles() -> Option<PathBuf> {
    // 1. Alongside the exe / in the bundle resource dir (production)
    if let Some(res) = resource_dir() {
        let p = res.join("profiles");
        if p.join("samsung_s25_ultra.json").exists() {
            return Some(p);
        }
    }

    // 2. Dev: CWD is src-tauri/ → ../profiles
    let dev1 = PathBuf::from("../profiles");
    if dev1.join("samsung_s25_ultra.json").exists() {
        return Some(dev1);
    }

    // 3. Dev: CWD is project root → profiles/
    let dev2 = PathBuf::from("profiles");
    if dev2.join("samsung_s25_ultra.json").exists() {
        return Some(dev2);
    }

    None
}
