use crate::adb_bridge;
use crate::sdk;
use std::path::PathBuf;

/// Apply common emulator detection bypasses via setprop
pub fn bypass_detection(sdk_path: &PathBuf, serial: &str) -> Result<String, String> {
    let props = [
        ("ro.debuggable", "0"),
        ("ro.secure", "1"),
        ("ro.build.tags", "release-keys"),
        ("ro.build.type", "user"),
        ("ro.build.selinux", "1"),
        ("init.svc.adbd", "stopped"), // Hide ADB root daemon
    ];

    let mut ok = 0;
    let mut failed = 0;

    for (key, value) in &props {
        match adb_bridge::setprop(sdk_path, serial, key, value) {
            Ok(_) => ok += 1,
            Err(_) => failed += 1,
        }
    }

    // Also stop adbd briefly and restart to hide root ADB
    let _ = adb_bridge::shell(sdk_path, serial, "stop adbd");
    std::thread::sleep(std::time::Duration::from_millis(500));
    let _ = adb_bridge::shell(sdk_path, serial, "start adbd");

    Ok(format!("Bypassed: {} props set, {} skipped", ok, failed))
}
