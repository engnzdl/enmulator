<p align="center">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey?style=for-the-badge" alt="Platform">
  <img src="https://img.shields.io/badge/built%20with-Tauri%202-ffc131?style=for-the-badge&logo=tauri" alt="Tauri">
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/license-GPLv3-blue?style=for-the-badge" alt="License">
</p>

<p align="center">
  <img src="https://img.shields.io/github/stars/engnzdl/enmulator?style=social" alt="Stars">
  <img src="https://img.shields.io/github/forks/engnzdl/enmulator?style=social" alt="Forks">
</p>

<h1 align="center">Enmulator</h1>
<p align="center"><strong>Cross-Platform Android Emulator Manager</strong></p>

<p align="center">
  <a href="https://github.com/engnzdl/enmulator/releases/latest/download/Enmulator_aarch64.dmg"><img src="https://img.shields.io/badge/download-macOS%20.dmg-000?style=for-the-badge&logo=apple" alt="macOS"></a>
  <a href="https://github.com/engnzdl/enmulator/releases/latest/download/Enmulator_x64_en-US.msi"><img src="https://img.shields.io/badge/download-Windows%20.msi-0078D6?style=for-the-badge&logo=windows" alt="Windows"></a>
  <a href="https://github.com/engnzdl/enmulator/releases/latest/download/enmulator_amd64.AppImage"><img src="https://img.shields.io/badge/download-Linux%20.AppImage-FCC624?style=for-the-badge&logo=linux" alt="Linux AppImage"></a>
  <a href="https://github.com/engnzdl/enmulator/releases/latest/download/enmulator_amd64.deb"><img src="https://img.shields.io/badge/download-Linux%20.deb-FCC624?style=for-the-badge&logo=linux" alt="Linux deb"></a>
</p>

<p align="center">
  <img src="screenshots/1-main.png" alt="Main Window" width="45%">
  <img src="screenshots/2-create-device-step-1.png" alt="Create Device" width="45%">
</p>
<p align="center">
  <img src="screenshots/3-create-device-step-2.png" alt="System Image" width="45%">
  <img src="screenshots/4-settings.png" alt="Settings" width="45%">
</p>

<p align="center">Create, manage, and fully customize Android Virtual Devices with a premium desktop UI.<br>Built with Tauri 2 + React + Rust. No Android Studio required.</p>

---

## Why Enmulator?

Managing Android emulators via command line is tedious. Android Studio is heavy and overkill for just running AVDs. MuMuPlayer is closed-source. Genymotion is expensive.

**Enmulator** gives you a native, cross-platform desktop app that wraps the Android SDK tools in a fast, modern interface — with fingerprint spoofing, device identity management, root access, per-device proxy, and a headless REST API for CI/CD automation.

### vs The Alternatives

| | Android Studio AVD | Genymotion | BlueStacks | MuMuPlayer | **Enmulator** |
|---|---|---|---|---|---|
| **Platform** | macOS, Win, Linux | macOS, Win, Linux | Win, macOS | macOS, Win | **macOS, Win, Linux** |
| **Size** | ~2 GB (IDE) | ~400 MB | ~500 MB | ~600 MB | **~15 MB** |
| **Price** | Free | **$479/yr** (Pro) | Free (ads) | Free (limited) | **Free · GPLv3** |
| **Open Source** | ✗ | ✗ | ✗ | ✗ | **✓** |
| **Fingerprint Profiles** | ✗ | ✗ (manual) | ✗ | ✓ presets | **66 built-in, 16 brands** |
| **Device Identity Panel** | ✗ | ✗ | ✗ | ✓ | **✓ IMEI + SIM + phone** |
| **Magisk Root** | ✗ | ✗ | ✓ built-in | ✓ built-in | **✓ one-click rootAVD** |
| **Writable System** | ✗ | ✗ | ✓ | ✓ | **✓ toggle** |
| **Batch Operations** | ✗ | ✓ (Pro) | ✓ | ✓ | **✓ start/stop/clone/APK** |
| **REST API** | ✗ | ✓ (Pro) | ✗ | ✗ | **✓ built-in Actix-web** |
| **Proxy per Device** | ✗ | ✓ | ✗ | ✗ | **✓ + CA cert installer** |
| **File Explorer** | ✗ | ✓ | ✗ | ✗ | **✓ dual-pane host↔device** |

---

## Features

### Device Management

- **Create** — 2-step wizard: pick a fingerprint profile + system image. Auto-downloads missing images. Auto-generated random device name.
- **Start / Stop** — cold boot (no snapshot save/load). Detects manual emulator closes via 5s polling.
- **Clone** — copies AVD directory, registers the clone with avdmanager, excludes snapshots.
- **Delete** — stops the emulator if running, unregisters from avdmanager, removes AVD data.
- **Batch** — multi-select mode: start, stop, delete, install APK across multiple devices simultaneously.
- **Drag & drop APK** — drop a `.apk` file onto a device card to install instantly.

### 66 Pre-Loaded Device Profiles · 16 Brands

| Brand | Models |
|---|---|
| **Samsung** | S25 Ultra, S24 Ultra, S24, S23 Ultra, A55, A54 5G, Galaxy Z Flip5 |
| **Google** | Pixel 9 Pro, 9, 9a, 8 Pro, 8 |
| **Xiaomi** | 15 Ultra, 15, 14 Pro, 14, 13 Ultra, MIX Fold 3 |
| **OnePlus** | 13, 12, Ace 3 |
| **OPPO** | Find X8 Pro, Find X6 Pro, Find N3, Find N3 Flip, Reno 12 Pro, Reno10 Pro+ |
| **Huawei** | Mate 60 Pro, Mate 60, P70 Pro, P60 Pro, P60, nova 11 Pro |
| **HONOR** | 200 Pro, Magic6 Pro, X9b |
| **vivo** | X100 Pro, X100, X Fold3 Pro, Y100 |
| **Sony** | Xperia 1 VI, 5 VI, 1 V, 5 V |
| **Motorola** | Razr 50 Ultra, Edge 50 Ultra, G54 5G, Moto G34 5G |
| **realme** | GT 7 Pro, GT 6, GT5 Pro, GT Neo5, 14 Pro+, 12 Pro+ |
| **Nothing** | Phone 3a, Phone 2 |
| **ASUS** | ROG Phone 9, Zenfone 11 Ultra |
| **SHARP** | AQUOS sense8, R8s Pro, wish2, zero6, AQUOS V |
| **Lenovo** | Legion Y90, Legion Phone Duel 2 Pro |
| **TCL** | 40 X |

Each profile includes: build fingerprint, device codename, resolution, DPI, **IMEI 1 & 2, MEID, phone number, SIM operator (MCC+MNC), SIM country, ICCID**.

### Device Identity Panel

Modelled after MuMuPlayer's Device tab. Available for every device in the right panel:

- **Preset picker** — select brand → model → auto-fills all identity fields from the bundled profile
- **Custom mode** — manually enter any field
- **IMEI generator** — ↻ button generates a new Luhn-valid 15-digit IMEI
- **Fields:** IMEI 1 & 2, Phone Number, SIM Operator (code + display name), SIM Country
- **Apply** — pushes all fields to the running device via `adb root` + `setprop`

### Root & System Access

| Feature | How it works | Requirement |
|---|---|---|
| **ADB Root** | `adb root` — restarts adbd as root | `google_apis` (non-Play Store) image |
| **Magisk Root** | rootAVD patches ramdisk.img with Magisk | Device must be stopped; any image |
| **System RO/RW** | `adb root` + `adb remount` toggle | ADB root active; runtime only |

> **Tip:** For apps that require `su` (e.g. testing root-requiring flows), use **Magisk Root**. For quick ADB access, **ADB Root** is sufficient.

### Quick Actions

- **📍 GPS** — set custom coordinates via popup (default: Istanbul)
- **📁 Files** — dual-pane file explorer (host ↔ device). Browse, upload, download files. Host panel supports full directory navigation.
- **🔒 ADB Root / 🔓 Unroot** — toggle `adb root` (Google APIs images). Clear error message when image doesn't support it.
- **💾 System RO / System RW** — toggle `/system` writable (requires ADB root). Runtime only — resets on reboot.
- **🧲 Magisk Root** — patch the system image's ramdisk.img with Magisk via bundled rootAVD script.
- **🛡️ Bypass Detection** — set `ro.build.tags`, `ro.secure`, `ro.debuggable` props to production values.
- **ADB Serial** — shows `emulator-{port}` string when device is running for quick `adb -s` access.

### Per-Device Proxy

- HTTP proxy host + port per device
- Enable/disable toggle (applies via `adb shell settings put global http_proxy`)
- **Certificate installer** — picks a `.crt/.pem` file, computes the correct `<subject_hash_old>.0` filename using the device's openssl, installs to `/system/etc/security/cacerts/` (requires ADB root + writable system)

### Headless REST API

Built-in Actix-web server, configurable port (default: 8080). Start from Settings or via `auto_start_api`. All responses are JSON.

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/devices` | List all devices |
| `POST` | `/api/devices` | Create a new device |
| `POST` | `/api/devices/{id}/start` | Start (headless) |
| `POST` | `/api/devices/{id}/stop` | Stop |
| `DELETE` | `/api/devices/{id}` | Delete |
| `POST` | `/api/devices/{id}/clone` | Clone |
| `POST` | `/api/devices/{id}/adb` | Run ADB shell command |
| `GET` | `/api/devices/{id}/logcat` | SSE logcat stream |
| `GET` | `/api/profiles` | List fingerprint profiles |

<details>
<summary>Example: create + start a device</summary>

```bash
# Create
curl -X POST http://localhost:8080/api/devices \
  -H "Content-Type: application/json" \
  -d '{"name":"Test","api_level":34,"abi":"x86_64","tag":"google_apis"}'

# Start
curl -X POST http://localhost:8080/api/devices/test/start
# → {"port":5554}

# ADB shell
curl -X POST http://localhost:8080/api/devices/test/adb \
  -H "Content-Type: application/json" \
  -d '{"cmd":"getprop ro.build.version.release"}'
# → {"output":"14\n"}

# Logcat (SSE)
curl -N http://localhost:8080/api/devices/test/logcat
```

All errors return `4xx` with `{"success": false, "error": "..."}`.

</details>

### Settings

Configurable via ⚙ gear icon. **Settings take effect immediately** (no restart needed):

| Setting | Default | Description |
|---|---|---|
| `sdk_path` | auto-detect | Android SDK root |
| `devices_dir` | platform data dir | Where AVD data is stored |
| `api_server_port` | `8080` | REST API port |
| `default_headless` | `false` | Start without emulator window |
| `auto_start_api` | `false` | Auto-start REST API on launch |
| `default_api_level` | `34` | Preferred Android version |
| `default_abi` | `x86_64` | Preferred CPU architecture |
| `default_tag` | `google_apis` | Preferred system image variant |

---

## Tech Stack

| Layer | Technology |
|---|---|
| **Desktop Shell** | [Tauri 2](https://v2.tauri.app/) (Rust) |
| **Frontend** | React 18 + TypeScript + Vite 5 |
| **Backend** | Rust — 13 modules, 43 Tauri IPC commands |
| **REST API** | Actix-web 4 (separate thread, shared state) |
| **Styling** | CSS custom properties, Inter font, premium dark theme |
| **Android** | SDK command-line tools (avdmanager, sdkmanager, adb, emulator) |
| **Root** | rootAVD + Magisk (bundled) |

### Rust Modules

```
src-tauri/src/
├── main.rs          43 Tauri command handlers, batch ops, cert installer
├── emulator.rs      Process lifecycle, port management (5554+ auto-assign)
├── sdk.rs           SDK detection, platform-aware binary names, sdkmanager wrapper
├── avd_manager.rs   AVD creation, clone registration, config.ini overrides
├── adb_bridge.rs    shell, setprop, install_apk, push, pull
├── fingerprint.rs   Profile CRUD + apply_to_device (setprop identity)
├── device.rs        Device struct + Arc<Mutex> persistent store
├── config.rs        App config (Arc<Mutex<Config>>, live updates)
├── paths.rs         Cross-platform dirs + resource bundle discovery
├── extras.rs        Screen recording, GPS, clipboard sync, logcat
├── bypass.rs        Emulator detection bypass
├── set_proxy.rs     Per-device HTTP proxy store
├── api_server.rs    Actix-web REST server (shared Arc<Mutex<Config>>)
└── build.rs         Tauri build script
```

### React Components

```
src/components/
├── DeviceCard.tsx          Sidebar card: status dot, drag-drop APK
├── CreateWizard.tsx        2-step wizard with live download progress bar
├── DeviceIdentityPanel.tsx Brand/model picker, IMEI generator, SIM fields
├── QuickActions.tsx        GPS, Files, ADB Root, System RW, Magisk, Bypass
├── ProxyCard.tsx           Proxy toggle + CA cert installer
├── FileExplorer.tsx        Dual-pane file browser (host nav + device nav)
└── SettingsPanel.tsx       SDK path, defaults, headless, API server
```

---

## Quick Start

### Prerequisites

- [Android SDK Command-line Tools](https://developer.android.com/studio#command-line-tools-only) (or Android Studio)
- At least one downloaded system image (the app can download them for you)
- [Rust](https://rustup.rs/) + [Node.js 20+](https://nodejs.org/) *(for building from source only)*

### Install from Release

Download the installer for your platform from [Releases](https://github.com/engnzdl/enmulator/releases/latest) and run it.

### Build from Source

```bash
git clone --recurse-submodules https://github.com/engnzdl/enmulator.git
cd enmulator
npm install
npm run tauri dev
```

### First Launch

1. The app auto-detects your Android SDK (`ANDROID_HOME` / `ANDROID_SDK_ROOT` / platform default paths). If not found → Settings ⚙ → SDK Path → Browse.
2. Click **+ New Device** → pick a device profile (optional, sets resolution/DPI/identity) → select system image → **Create Device**. Missing images are downloaded automatically.
3. Click **▶ Start** on any device card. Cold boot takes ~30–60s. Identity (IMEI, operator, phone number) is auto-applied after boot completes.
4. Use the **Device Identity** panel to change identity on the fly while the device is running.
5. Use **Quick Actions** for GPS, file transfer, root, writable system, or Magisk root.

---

## Platform Notes

| Platform | Notes |
|---|---|
| **macOS** | Apple Silicon (ARM64) and Intel (x86_64) builds available. KVM not applicable. |
| **Windows** | Requires [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 11). HAXM recommended for hardware acceleration. |
| **Linux** | KVM is auto-enabled when `/dev/kvm` exists. Run `sudo usermod -aG kvm $USER` if needed. |

### System Requirements

- **macOS:** 12.0+ (Monterey)
- **Windows:** 10+ (x64)
- **Linux:** Ubuntu 20.04+ / Debian 11+ / Fedora 36+ with `libwebkit2gtk-4.1` and `libgtk-3`
- **RAM:** 8 GB+ recommended (each emulator uses 2–4 GB)
- **Storage:** SSD recommended for AVD disk images

---

## Limitations

- **`ro.product.*` properties** (brand, model, manufacturer) are immutable — set by the system image, not the profile. Only mutable props (IMEI, SIM, phone) are applied via `setprop`.
- **`adb root`** only works on `google_apis` (non-Play Store) images. For Play Store images, use **Magisk Root**.
- **Writable system** (`adb remount`) is runtime-only — changes reset on reboot. Affected by dm-verity on API 29+.
- **Magisk Root** patches the shared ramdisk.img — all AVDs using the same system image are affected.
- **ARM64 system images** only run natively on Apple Silicon. Use `x86_64` on Intel Macs, Windows, and Linux.
- **Wayland** — headless mode works everywhere; windowed mode requires XWayland on Wayland-only desktops.

---

## Building

```bash
# Development
npm run tauri dev

# Production bundle
npm run tauri build
```

| Platform | Output |
|---|---|
| **macOS** | `src-tauri/target/release/bundle/dmg/Enmulator_*.dmg` |
| **Windows** | `src-tauri/target/release/bundle/msi/Enmulator_*.msi` |
| **Linux** | `…/deb/enmulator_*.deb` + `…/appimage/*.AppImage` |

CI builds run automatically on tag push via GitHub Actions (macOS ARM64 + x86_64, Windows x64, Linux x64).

---

## IPC Command Reference

<details>
<summary>All 43 Tauri commands</summary>

| Command | Args | Returns |
|---|---|---|
| `detect_sdk_cmd` | — | SDK path |
| `list_available_images_cmd` | — | `Vec<SystemImage>` |
| `install_system_image_cmd` | `package` | streams `download-progress` |
| `list_devices` | — | `Vec<Device>` |
| `create_device` | `name, api_level, abi, tag, fingerprint_profile?` | `Device` |
| `delete_device` | `id` | — |
| `clone_device` | `source_id, target_name` | `Device` |
| `start_device` | `id, headless` | `port: u16` |
| `stop_device` | `id` | — |
| `check_device_alive` | `id` | `bool` |
| `batch_start` | `ids` | `BatchResult` |
| `batch_stop` | `ids` | `BatchResult` |
| `batch_delete` | `ids` | `BatchResult` |
| `adb_shell` | `id, cmd` | `String` |
| `install_apk` | `id, apk_path` | `String` |
| `set_device_proxy` | `id, host, port, enabled` | — |
| `enable_proxy` | `id` | `ProxyConfig?` |
| `list_profiles` | — | `Vec<FingerprintProfile>` |
| `create_profile` | `profile` | `FingerprintProfile` |
| `delete_profile` | `name` | — |
| `apply_profile` | `device_id, profile_name` | — |
| `set_device_identity` | `device_id, imei?, imei2?, meid?, phone_number?, sim_operator?, sim_operator_name?, sim_country?, sim_serial?` | — |
| `toggle_root` | `id` | `String` |
| `toggle_writable_system` | `id` | `String` |
| `root_with_magisk` | `id` | `String` |
| `bypass_detection` | `id` | `String` |
| `start_screen_record` | `id` | — |
| `stop_screen_record` | `id` | `String` (local path) |
| `clipboard_sync` | `id, direction, text?` | `String` |
| `gps_set` | `id, lat, lon` | — |
| `logcat_start` | `id` | streams `logcat-line` |
| `start_api_server` | `port` | `String` |
| `stop_api_server` | — | `String` |
| `get_config` | — | `Config` |
| `set_sdk_path` | `path` | — |
| `update_config` | `sdk_path?, devices_dir?, api_server_port?, default_headless?, auto_start_api?, default_api_level?, default_abi?, default_tag?` | — |
| `list_device_templates` | — | `Vec<String>` |
| `list_files` | `id, path` | `Vec<FileEntry>` |
| `list_host_files` | `path` | `Vec<FileEntry>` |
| `pull_file` | `id, remote_path, local_path` | — |
| `push_file` | `id, local_path, remote_path` | — |
| `install_cert` | `id, cert_path` | `String` |

</details>

---

## Roadmap

- [x] Cross-platform build (macOS ARM64 + x86_64, Windows, Linux)
- [x] Device Identity Panel (brand/model picker, IMEI generator)
- [x] Magisk Root via bundled rootAVD
- [x] Writable System toggle
- [x] 66 device profiles across 16 brands
- [x] Headless REST API with SSE logcat
- [x] Dual-pane file explorer
- [x] Batch operations
- [ ] App install automation (batch APK from folder)
- [ ] Device group presets (save/load sets of devices)
- [ ] Screenshot capture
- [ ] Network throttle / latency simulation
- [ ] More API endpoints (proxy, identity, recording via REST)

---

## Contributing

GPLv3 licensed. Pull requests welcome.

```bash
git clone --recurse-submodules https://github.com/engnzdl/enmulator.git
cd enmulator
npm install
npm run tauri dev
```

Before submitting a PR, ensure both pass:

```bash
npx tsc --noEmit        # TypeScript
cargo build --manifest-path src-tauri/Cargo.toml  # Rust
```

---

## License

GNU General Public License v3.0

© 2025 [engnzdl](https://github.com/engnzdl)

Free software — use, modify, and distribute under GPL v3. Derivative works must also be open-sourced under the same license.

---

<p align="center">
  <sub>Built with ❤ by <a href="https://github.com/engnzdl">engnzdl</a></sub>
</p>
