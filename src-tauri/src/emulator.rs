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
    pub fn new() -> Self {
        Self { processes: Mutex::new(HashMap::new()), next_port: Mutex::new(5554) }
    }

    pub fn start(&self, sdk_path: &PathBuf, avd_name: &str, headless: bool) -> Result<u16, String> {
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

        let child = cmd.spawn().map_err(|e| format!("Failed to start: {}", e))?;
        self.processes.lock().unwrap().insert(avd_name.to_string(), child);
        Ok(port)
    }

    pub fn stop(&self, sdk_path: &PathBuf, avd_name: &str, port: u16) {
        let adb = sdk::get_adb_path(sdk_path);
        let serial = format!("emulator-{}", port);
        let _ = Command::new(&adb).args(["-s", &serial, "emu", "kill"]).output();
        if let Some(mut child) = self.processes.lock().unwrap().remove(avd_name) {
            let _ = child.kill();
        }
    }
}
