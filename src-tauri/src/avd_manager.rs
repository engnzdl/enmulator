use crate::sdk;
use crate::device::Device;
use std::path::PathBuf;
use std::process::Command;

pub fn create_avd(
    sdk_path: &PathBuf,
    device_id: &str,
    display_name: &str,
    api_level: u8,
    abi: &str,
    tag: &str,
    device_definition: &str,
    fingerprint_profile: Option<String>,
    devices_dir: &PathBuf,
) -> Result<Device, String> {
    let avd_name = format!("enmulator_{}", device_id);
    let avd_path = devices_dir.join(device_id);
    let avdmanager = sdk::get_avdmanager_path(sdk_path);

    let package = format!("system-images;android-{};{};{}", api_level, tag, abi);

    let output = Command::new(&avdmanager)
        .args([
            "create", "avd", "--force",
            "--name", &avd_name,
            "--package", &package,
            "--tag", tag,
            "--abi", abi,
            "--device", device_definition,
            "--path", avd_path.to_str().unwrap_or("/tmp"),
        ])
        .output()
        .map_err(|e| format!("avdmanager error: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(Device {
        id: device_id.to_string(),
        display_name: display_name.to_string(),
        avd_name,
        profile: Some(device_definition.to_string()),
        fingerprint_profile,
        api_level,
        status: "stopped".to_string(),
        port: 0,
        root_enabled: false,
        adb_enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn list_device_definitions(sdk_path: &PathBuf) -> Result<Vec<String>, String> {
    let avdmanager = sdk::get_avdmanager_path(sdk_path);
    let output = Command::new(&avdmanager)
        .args(["list", "device", "-c"])
        .output()
        .map_err(|e| format!("avdmanager error: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}
