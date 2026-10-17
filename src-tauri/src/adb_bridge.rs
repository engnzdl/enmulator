use crate::sdk;
use crate::paths;
use std::path::PathBuf;

pub fn shell(sdk_path: &PathBuf, serial: &str, cmd: &str) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "shell", cmd])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn setprop(sdk_path: &PathBuf, serial: &str, key: &str, value: &str) -> Result<(), String> {
    let escaped = value.replace('\'', "'\\''");
    shell(sdk_path, serial, &format!("setprop {} '{}'", key, escaped)).map(|_| ())
}

/// Returns true if Magisk (with `su`) is available on the device.
/// When Magisk is installed, `su -c 'magisk -v'` prints a version string.
pub fn has_magisk(sdk_path: &PathBuf, serial: &str) -> bool {
    shell(sdk_path, serial, "su -c 'magisk -v'")
        .map(|o| o.trim().chars().any(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

/// Set a property via Magisk `resetprop` — works on any system image,
/// including read-only `ro.*` props that plain `setprop` cannot touch.
pub fn magisk_resetprop(sdk_path: &PathBuf, serial: &str, key: &str, value: &str) -> Result<(), String> {
    let v = value.replace('"', "\\\"");
    shell(sdk_path, serial, &format!("su -c 'magisk resetprop {} \"{}\"'", key, v)).map(|_| ())
}

/// Run a Magisk denylist CLI command (`enable`, `disable`, `add PKG [PROC]`, `ls`, `rm`).
pub fn magisk_denylist(sdk_path: &PathBuf, serial: &str, args: &str) -> Result<String, String> {
    su_shell(sdk_path, serial, &format!("magisk --denylist {}", args))
}

/// Enable Zygisk in the Magisk database (DenyList enforcement is Zygisk-based
/// and does nothing without it). Writes the SQL to a temp script on-device to
/// avoid nested shell quoting issues with the SQL single quotes.
pub fn magisk_enable_zygisk(sdk_path: &PathBuf, serial: &str) -> Result<(), String> {
    let script = "magisk --sqlite \"INSERT OR REPLACE INTO settings (key,value) VALUES ('zygisk',1)\"";
    let local = std::env::temp_dir().join("enmulator_zygisk.sh");
    std::fs::write(&local, script).map_err(|e| e.to_string())?;
    let push_res = push(sdk_path, serial, &local.to_string_lossy(), "/data/local/tmp/enmulator_zygisk.sh");
    let _ = std::fs::remove_file(&local);
    push_res?;
    su_shell(sdk_path, serial, "sh /data/local/tmp/enmulator_zygisk.sh").map(|_| ())
}

/// Install a Magisk module from a zip already pushed to the device.
pub fn magisk_install_module(sdk_path: &PathBuf, serial: &str, remote_zip: &str) -> Result<String, String> {
    su_shell(sdk_path, serial, &format!("magisk --install-module {}", remote_zip))
}

/// Run a command as root via Magisk `su`.
pub fn su_shell(sdk_path: &PathBuf, serial: &str, cmd: &str) -> Result<String, String> {
    let escaped = cmd.replace('\'', "'\\''");
    shell(sdk_path, serial, &format!("su -c '{}'", escaped))
}

pub fn install_apk(sdk_path: &PathBuf, serial: &str, apk_path: &str) -> Result<String, String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "install", "-r", apk_path])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn push(sdk_path: &PathBuf, serial: &str, local: &str, remote: &str) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "push", local, remote])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Capture a PNG screenshot from the device into a local file.
pub fn screenshot(sdk_path: &PathBuf, serial: &str, local_path: &str) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "exec-out", "screencap", "-p"])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        std::fs::write(local_path, &output.stdout).map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Configure Magisk root hiding end-to-end: enable Zygisk, enable DenyList,
/// hide root from Google Play Services / Play Store / GSF + the given packages,
/// and install NoHello (Zygisk trace hiding). A reboot is required afterward.
pub fn hide_root_core(sdk_path: &PathBuf, serial: &str, packages: &[String]) -> Result<String, String> {
    if !has_magisk(sdk_path, serial) {
        return Err("Magisk not detected — root with Magisk first".into());
    }
    magisk_enable_zygisk(sdk_path, serial)?;
    magisk_denylist(sdk_path, serial, "enable")?;

    let mut targets: Vec<(String, String)> = vec![
        ("com.google.android.gms".to_string(), "com.google.android.gms.unstable".to_string()),
        ("com.google.android.gms".to_string(), "com.google.android.gms".to_string()),
        ("com.android.vending".to_string(), "com.android.vending".to_string()),
        ("com.google.android.gsf".to_string(), "com.google.android.gsf".to_string()),
    ];
    for p in packages {
        let p = p.trim();
        if !p.is_empty() {
            targets.push((p.to_string(), p.to_string()));
        }
    }

    let mut added = 0usize;
    for (pkg, proc) in &targets {
        if magisk_denylist(sdk_path, serial, &format!("add {} {}", pkg, proc)).is_ok() {
            added += 1;
        }
    }

    // NoHello (Zygisk trace hiding) — best-effort.
    let res_dir = paths::resource_dir();
    let exe_dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let mut candidates: Vec<PathBuf> = vec![
        PathBuf::from("../src-tauri/NoHello.zip"),
        PathBuf::from("src-tauri/NoHello.zip"),
        exe_dir.join("NoHello.zip"),
    ];
    if let Some(ref rd) = res_dir {
        candidates.push(rd.join("NoHello.zip"));
    }
    let nohello_note = match candidates.iter().find(|p| p.exists()) {
        Some(zip) => {
            let remote = "/data/local/tmp/NoHello.zip";
            match push(sdk_path, serial, &zip.to_string_lossy(), remote)
                .and_then(|_| magisk_install_module(sdk_path, serial, remote))
            {
                Ok(_) => "NoHello installed (hides Zygisk traces).".to_string(),
                Err(e) => format!("NoHello install failed: {}", e),
            }
        }
        None => "NoHello.zip not bundled — plain DenyList only.".to_string(),
    };

    let ls = magisk_denylist(sdk_path, serial, "ls").unwrap_or_default();
    Ok(format!(
        "Root hiding configured — {} denylist targets (Zygisk enabled). {}\n\
         ⚠️  REBOOT required: Zygisk + DenyList + NoHello take effect on next boot.\n\n{}",
        added,
        nohello_note,
        ls
    ))
}

pub fn pull(sdk_path: &PathBuf, serial: &str, remote: &str, local: &str) -> Result<(), String> {
    let adb = sdk::get_adb_path(sdk_path);
    let output = sdk::new_command(&adb)
        .args(["-s", serial, "pull", remote, local])
        .output()
        .map_err(|e| format!("ADB error: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

