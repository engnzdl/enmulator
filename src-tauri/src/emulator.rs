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

        // Pass identity props at launch so ro.* properties take effect
        if let Some(p) = profile {
            cmd.arg("-prop").arg(format!("ro.product.brand={}", p.brand));
            cmd.arg("-prop").arg(format!("ro.product.manufacturer={}", p.manufacturer));
            cmd.arg("-prop").arg(format!("ro.product.model={}", p.model));
            cmd.arg("-prop").arg(format!("ro.product.device={}", p.device));
            cmd.arg("-prop").arg(format!("ro.product.name={}", p.device));
            cmd.arg("-prop").arg(format!("ro.build.fingerprint={}", p.fingerprint));
            if !p.imei.is_empty() {
                cmd.arg("-prop").arg(format!("persist.radio.imei={}", p.imei));
            }
            if !p.imei2.is_empty() {
                cmd.arg("-prop").arg(format!("persist.radio.imei2={}", p.imei2));
            }
            if !p.meid.is_empty() {
                cmd.arg("-prop").arg(format!("persist.radio.meid={}", p.meid));
            }
            if !p.sim_operator.is_empty() {
                cmd.arg("-prop").arg(format!("gsm.sim.operator.numeric={}", p.sim_operator));
                cmd.arg("-prop").arg(format!("gsm.operator.numeric={}", p.sim_operator));
            }
            if !p.sim_operator_name.is_empty() {
                cmd.arg("-prop").arg(format!("gsm.sim.operator.alpha={}", p.sim_operator_name));
                cmd.arg("-prop").arg(format!("gsm.operator.alpha={}", p.sim_operator_name));
            }
            if !p.sim_country.is_empty() {
                cmd.arg("-prop").arg(format!("gsm.sim.operator.iso-country={}", p.sim_country));
                cmd.arg("-prop").arg(format!("gsm.operator.iso-country={}", p.sim_country));
            }
            if !p.phone_number.is_empty() {
                cmd.arg("-prop").arg(format!("gsm.sim.phone_number={}", p.phone_number));
            }
            if !p.sim_serial.is_empty() {
                cmd.arg("-prop").arg(format!("gsm.sim.serial={}", p.sim_serial));
            }
        }

        let child = cmd.spawn().map_err(|e| format!("Failed to start: {}", e))?;
        self.processes.lock().unwrap().insert(avd_name.to_string(), child);
        Ok(port)
    }

    pub fn stop(&self, sdk_path: &PathBuf, avd_name: &str, port: u16) {
        let adb = sdk::get_adb_path(sdk_path);
        let serial = format!("emulator-{}", port);
        // Delete quick-boot snapshot so next start is cold
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
