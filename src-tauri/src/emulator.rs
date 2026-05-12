use crate::fingerprint::FingerprintProfile;
use crate::sdk;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;

pub struct EmulatorStore {
    pub processes: Mutex<HashMap<String, Child>>,
    next_port: Mutex<u16>,
}

impl EmulatorStore {
    pub fn new(existing_devices: &[crate::device::Device]) -> Self {
        let max_port = existing_devices
            .iter()
            .map(|d| d.port)
            .filter(|p| *p > 0)
            .max()
            .unwrap_or(5552);
        Self {
            processes: Mutex::new(HashMap::new()),
            next_port: Mutex::new(max_port.saturating_add(2)),
        }
    }

    pub fn start(
        &self,
        sdk_path: &PathBuf,
        avd_name: &str,
        headless: bool,
        profile: Option<&FingerprintProfile>,
    ) -> Result<u16, String> {
        let emulator = sdk::get_emulator_path(sdk_path);
        let port = {
            let mut p = self.next_port.lock().unwrap();
            let current = *p;
            *p += 2;
            current
        };

        let mut cmd = Command::new(&emulator);
        cmd.arg("-avd").arg(avd_name).arg("-port").arg(port.to_string());
        if headless { cmd.arg("-no-window"); }
        cmd.arg("-no-boot-anim");
        cmd.arg("-no-snapshot-save");
        cmd.arg("-no-snapshot-load");

        // Enable KVM hardware acceleration on Linux
        #[cfg(target_os = "linux")]
        if std::path::Path::new("/dev/kvm").exists() {
            cmd.arg("-accel").arg("on");
        }

        let _ = profile;

        let child = cmd.spawn().map_err(|e| format!("Failed to start: {}", e))?;
        self.processes.lock().unwrap().insert(avd_name.to_string(), child);
        Ok(port)
    }

    pub fn stop(&self, sdk_path: &PathBuf, avd_name: &str, port: u16) {
        let adb = sdk::get_adb_path(sdk_path);
        let serial = format!("emulator-{}", port);
        let _ = Command::new(&adb)
            .args(["-s", &serial, "emu", "avd", "snapshot", "delete", "default_boot"])
            .output();
        let _ = Command::new(&adb).args(["-s", &serial, "emu", "kill"]).output();
        if let Some(mut child) = self.processes.lock().unwrap().remove(avd_name) {
            let _ = child.kill();
        }
    }

    pub fn is_alive(sdk_path: &PathBuf, port: u16) -> bool {
        let adb = sdk::get_adb_path(sdk_path);
        let serial = format!("emulator-{}", port);
        if let Ok(output) = Command::new(&adb).args(["devices"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains(&serial);
        }
        false
    }
}
