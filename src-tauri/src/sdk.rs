use std::path::PathBuf;

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
    sdk.join("cmdline-tools/latest/bin/avdmanager")
}

pub fn get_emulator_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("emulator/emulator")
}

pub fn get_adb_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("platform-tools/adb")
}

pub fn get_sdkmanager_path(sdk: &PathBuf) -> PathBuf {
    sdk.join("cmdline-tools/latest/bin/sdkmanager")
}
