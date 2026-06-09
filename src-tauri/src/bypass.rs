use crate::adb_bridge;
use crate::sdk;
use std::path::PathBuf;

/// Apply emulator detection bypasses.
///
/// Strategy:
/// 1. Try setprop for ro.* props (works on userdebug/eng images)
/// 2. If system is writable (adb remount was done), patch /system/build.prop directly
///    — this is the only method that reliably works on google_apis images
pub fn bypass_detection(sdk_path: &PathBuf, serial: &str) -> Result<String, String> {
    let _ = adb_bridge::shell(sdk_path, serial, "root");

    let props = [
        ("ro.debuggable", "0"),
        ("ro.secure", "1"),
        ("ro.build.tags", "release-keys"),
        ("ro.build.type", "user"),
        ("ro.build.selinux", "1"),
    ];

    let mut setprop_ok = 0;
    for (key, value) in &props {
        if adb_bridge::setprop(sdk_path, serial, key, value).is_ok() {
            setprop_ok += 1;
        }
    }

    // Check if /system is writable (writable system mode was enabled)
    let writable = adb_bridge::shell(sdk_path, serial,
        "touch /system/enmulator_test 2>/dev/null && echo writable || echo readonly")
        .map(|o| o.trim().contains("writable"))
        .unwrap_or(false);

    if writable {
        // Clean up test file
        let _ = adb_bridge::shell(sdk_path, serial, "rm -f /system/enmulator_test");

        // Patch build.prop directly — the only reliable method on google_apis images
        let patch_result = patch_build_prop(sdk_path, serial);
        match patch_result {
            Ok(patched) => {
                return Ok(format!(
                    "Bypass applied via build.prop ✓\n\
                    Patched {} properties in /system/build.prop\n\
                    Reboot the device for changes to take effect.",
                    patched
                ));
            }
            Err(e) => {
                return Err(format!("build.prop patch failed: {}", e));
            }
        }
    }

    if setprop_ok > 0 {
        Ok(format!("Bypass applied: {} props set via setprop.", setprop_ok))
    } else {
        Err(
            "Bypass requires writable system.\n\n\
            Steps:\n\
            1. Enable 'ADB Root' in Quick Actions\n\
            2. Enable 'System RW' in Quick Actions\n\
            3. Then run Bypass Detection again"
            .to_string()
        )
    }
}

/// Patch /system/build.prop to hide emulator fingerprints.
fn patch_build_prop(sdk_path: &PathBuf, serial: &str) -> Result<usize, String> {
    // Read current build.prop
    let content = adb_bridge::shell(sdk_path, serial, "cat /system/build.prop")?;

    let replacements: &[(&str, &str)] = &[
        ("ro.debuggable=", "ro.debuggable=0"),
        ("ro.secure=", "ro.secure=1"),
        ("ro.build.tags=", "ro.build.tags=release-keys"),
        ("ro.build.type=", "ro.build.type=user"),
        ("ro.build.selinux=", "ro.build.selinux=1"),
    ];

    let mut new_lines: Vec<String> = Vec::new();
    let mut patched = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();
        let mut replaced = false;
        for (prefix, new_line) in replacements {
            if trimmed.starts_with(prefix) {
                new_lines.push(new_line.to_string());
                patched += 1;
                replaced = true;
                break;
            }
        }
        if !replaced {
            new_lines.push(line.to_string());
        }
    }

    let new_content = new_lines.join("\n");

    // Write via temp file to avoid shell escaping nightmares
    let adb = sdk::get_adb_path(sdk_path);
    let tmp_local = std::env::temp_dir().join("enmulator_build.prop");
    std::fs::write(&tmp_local, &new_content).map_err(|e| e.to_string())?;

    // Push to /sdcard/ then copy to /system/build.prop
    std::process::Command::new(&adb)
        .args(["-s", serial, "push", &tmp_local.to_string_lossy(), "/sdcard/build.prop"])
        .output()
        .map_err(|e| e.to_string())?;

    adb_bridge::shell(sdk_path, serial,
        "cp /sdcard/build.prop /system/build.prop && chmod 644 /system/build.prop && rm /sdcard/build.prop")?;

    let _ = std::fs::remove_file(&tmp_local);
    Ok(patched)
}
