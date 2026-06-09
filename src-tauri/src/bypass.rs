use crate::adb_bridge;
use std::path::PathBuf;

/// Apply common emulator detection bypasses via setprop.
/// Note: ro.* properties on Android 8+ require a writable system partition
/// (e.g. adb root + adb remount on a userdebug/eng image). On production
/// emulator images these will be counted as skipped.
pub fn bypass_detection(sdk_path: &PathBuf, serial: &str) -> Result<String, String> {
    // Require adb root first so setprop has the best chance of working
    let _ = adb_bridge::shell(sdk_path, serial, "root");

    let props = [
        ("ro.debuggable", "0"),
        ("ro.secure", "1"),
        ("ro.build.tags", "release-keys"),
        ("ro.build.type", "user"),
        ("ro.build.selinux", "1"),
    ];

    let mut ok = 0;
    let mut failed = 0;

    for (key, value) in &props {
        match adb_bridge::setprop(sdk_path, serial, key, value) {
            Ok(_) => ok += 1,
            Err(_) => failed += 1,
        }
    }

    Ok(format!("Bypass applied: {} props set, {} skipped (ro.* props require userdebug image)", ok, failed))
}
