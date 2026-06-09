# Changelog

All notable changes to Enmulator.

## [0.2.0] — 2026-06-09

### New Features

- **Device Identity Panel** — MuMuPlayer-style Device tab in the right panel. Brand/model preset picker (from bundled profiles), auto-fill all identity fields, IMEI random generator (Luhn-valid), SIM operator/country fields, inline Apply button. Works on any running device via `adb setprop`.
- **Writable System toggle** — `adb root` + `adb remount` as a Quick Action button (System RO/RW). State shown in device meta bar. Runtime only — resets on reboot.
- **Magisk Root** — one-click root via bundled rootAVD script. Patches the system image's `ramdisk.img` with Magisk. Device must be stopped. Works on any system image variant.
- **ADB serial display** — `emulator-{port}` shown in device panel when running. Copy-paste ready for `adb -s` commands.
- **Host file explorer** — FileExplorer host panel now fully functional. Directory navigation (Browse + Up button), file listing, click-to-navigate, upload selected file to device.
- **Global error toast** — start/stop/delete/clone/root errors now shown as a 5-second dismissable banner in the UI instead of being silently swallowed.
- **Quick action feedback** — success/error messages shown inline below each action button.
- **Duplicate device name detection** — `create_device` and `clone_device` now reject names that normalize to an existing device ID.

### Profile Expansion: 30 → 66 profiles · 10 → 16 brands

New brands: **vivo** (4), **HONOR** (3), **SHARP** (5), **Lenovo** (2), **TCL** (1)

New models in existing brands:
| Brand | Added |
|---|---|
| Samsung | Galaxy Z Flip5, Galaxy A54 5G |
| OnePlus | Ace 3 |
| Xiaomi | 14 Pro, 13 Ultra, MIX Fold 3 |
| realme | GT5 Pro, GT Neo5, 12 Pro+ |
| OPPO | Find N3, Find N3 Flip, Find X6 Pro, Reno10 Pro+ |
| Motorola | G54 5G, Moto G34 5G |
| Sony | Xperia 1 V, Xperia 5 V |
| Google | Pixel 8 |
| Huawei | Mate 60, P60, nova 11 Pro |

### Bug Fixes

**Critical:**
- `adb_bridge::push()` always returned `Ok(())` even on failure — silent data loss on file upload fixed
- `clone_device` — cloned AVDs were never registered with avdmanager → clones couldn't start (fixed: writes `~/.android/avd/<name>.ini` and updates `AvdId=` in `config.ini`)
- `start_device` — no double-start guard → starting a running device spawned a second emulator and corrupted port state
- `install_cert` — CA certificate was copied with its original filename; Android requires `<subject_hash_old>.0` naming (fixed: uses device's `openssl x509 -subject_hash_old` to compute correct name)
- `auto_start_api` config flag was never read in `main()` — REST API never auto-started
- `create_profile` — missing `imei`/`imei2`/SIM fields caused `invalid args` deserialization error from the wizard (fixed: fields are now `#[serde(default)]` in Rust + `optional` in TypeScript)
- `update_config` / `set_sdk_path` — only wrote to disk; in-memory `Config` state stayed stale → settings required app restart to take effect (fixed: `Config` wrapped in `Arc<Mutex<Config>>` shared across all Tauri commands and REST API server)

**High:**
- `bypass.rs` — `stop adbd` → adb connection dropped → `start adbd` never executed. Broken sequence removed. `init.svc.adbd` (meaningless prop) removed.
- `delete_device` — didn't stop running emulator first; didn't call `avdmanager delete avd` → orphaned `.ini` files
- `batch_delete` — missing `avdmanager delete avd` call → orphaned `.ini` files in `~/.android/avd/`
- `list_files` — symlink entries (`name -> target`) produced garbled filenames
- `list_files` — paths with spaces broke `ls -la {path}` (now quoted: `ls -la '{path}'`)
- `adb_bridge::setprop` — values with spaces/special characters were unquoted → shell parsing errors
- `App.tsx` auto-select — stale `selectedDeviceId` after batch delete not cleared; now selects first available device

**Medium:**
- `DeviceStore::new()` — devices loaded from JSON with `status: "running"` and `root_enabled: true` from previous session; emulator processes don't survive restarts → runtime state now reset on load
- `FileExplorer` host panel — `listHost()` was a no-op, `hostEntries` always empty; host-side file browsing was completely broken
- `create_device` / `clone_device` — raw user input used as device ID; `/`, `.`, `&`, spaces etc. broke avdmanager or path creation (fixed: `sanitize_id()` allows only `[a-z0-9-_]`, collapses `_`, trims edges)

### Cross-Platform Fixes

- **Windows: `root_with_magisk`** — used `bash rootAVD.sh` (bash not available on Windows); now uses `cmd /c rootAVD.bat`
- **Windows: `register_avd_at_path`** — backslashes in AVD ini file broke Android SDK parsing; now normalized to forward slashes
- **Windows: reserved filenames** — `sanitize_id()` prefixes `dev_` to Windows reserved names (CON, NUL, COM1–9, LPT1–9)
- **All platforms: profiles not bundled** — `tauri.conf.json` had no `resources` field; production installs had no default profiles. Fixed: `profiles/` and `rootAVD/` bundled as resources
- **macOS: wrong resource path** — `ensure_default_profiles` used `Resources/profiles` (wrong); now uses `resource_dir()` helper that correctly resolves `../Resources/` for macOS bundles
- **CI: missing Rust target** — `dtolnay/rust-toolchain` now passes `targets: ${{ matrix.target }}` for cross-compilation

### Architecture

- `Config` → `Arc<Mutex<Config>>` managed state shared between all Tauri commands and the Actix REST server; live settings updates without restart
- `avd_manager::sanitize_id()` — safe, cross-platform device ID normalization
- `avd_manager::register_avd_at_path()` — AVD clone registration in `~/.android/avd/`
- `avd_manager::delete_avd()` — proper avdmanager cleanup on device delete
- `device.rs::set_writable_system()` — new store method
- `paths::resource_dir()` — platform-aware bundle resource directory resolution
- `list_host_files` Tauri command — host filesystem browsing via `std::fs::read_dir`
- `toggle_writable_system` Tauri command
- `root_with_magisk` Tauri command
- `DeviceIdentityPanel` React component
- **43 Tauri IPC commands** (was ~25)
- **8 React components** (was 7; added `DeviceIdentityPanel`)

### CI/CD

- Added macOS Intel (x86_64-apple-darwin) build via `macos-13` runner
- `fail-fast: false` — one platform failure doesn't cancel others
- `submodules: recursive` checkout for rootAVD
- `profiles/` and `rootAVD/` bundled into all platform installers

---

## [0.1.0] — 2026-05-12

### Added
- Cross-platform Android emulator manager (macOS, Windows, Linux)
- Tauri 2 + Rust backend with React + TypeScript frontend
- AVD creation with 2-step wizard (fingerprint profile + system image)
- 30 pre-loaded fingerprint profiles across 10 brands (Samsung, Pixel, Xiaomi, OnePlus, Huawei, Motorola, OPPO, Nothing, Asus, Sony, Realme)
- Per-profile identity with IMEI, phone number, SIM operator (auto-applied on boot via `adb root`)
- Device lifecycle: start, stop, clone, delete
- Batch operations: multi-select start, stop, delete, install APK
- Quick Actions: GPS (coordinate popup), File Explorer, Root toggle, Emu Detection Bypass
- Per-device HTTP proxy with certificate installer
- Auto port assignment (5554+) with restart-safe counter
- Cold boot only (no snapshot save/load, `-no-snapshot-save` + `-no-snapshot-load`)
- Live device status polling (detects manual emulator closes)
- Settings panel (⚙): SDK path, default API/ABI/tag, headless mode, API server port
- Headless REST API server (Actix-web, 9 endpoints) with SSE logcat stream
- KVM acceleration auto-detect on Linux
- Platform-native SDK binary paths (`.exe`/`.bat` on Windows)
- Platform config/data directories (`~/Library/Application Support`, `~/.config`, `%APPDATA%`)
- Auto-generated random device names
- Premium dark UI
- GPLv3 license

### Technical
- 13 Rust modules: main, emulator, sdk, adb_bridge, fingerprint, device, config, paths, avd_manager, extras, bypass, set_proxy, api_server
- 7 React components: App, DeviceCard, CreateWizard, QuickActions, ProxyCard, FileExplorer, SettingsPanel
- ~25 Tauri IPC commands
- Persistent device store (JSON)
- System image download with real-time progress streaming
- `adb root` support on Google APIs images
- Config.ini resolution/DPI overrides per profile
