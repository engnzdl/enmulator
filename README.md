# Enmulator

A modern desktop GUI for managing Android emulators. Built with Tauri (Rust backend) and React (TypeScript frontend).

## Features

- **Device Management** — Create, clone, delete, and list Android Virtual Devices (AVDs)
- **Start / Stop** — Launch emulators headless or with a window
- **Batch Operations** — Select multiple devices and start, stop, delete, or install APKs on all of them at once
- **ADB Shell** — Run arbitrary shell commands on any connected emulator
- **APK Install** — Drag-and-drop APK files onto device cards to install, or use batch install
- **Fingerprint Profiles** — Apply device fingerprint profiles to running emulators (brand, model, manufacturer, fingerprint)
- **Proxy Configuration** — Set or clear per-device HTTP proxy via ADB
- **Screen Recording** — Start/stop screen recording on a device and pull the resulting MP4
- **Clipboard Sync** — Get or set the clipboard content on a device
- **GPS Mocking** — Set mock GPS coordinates
- **Logcat Streaming** — Stream real-time logcat output as Tauri events
- **File Explorer** — Browse, pull, and push files on a device
- **Snapshot Management** — Save, load, list, and delete emulator snapshots
- **API Server** — Optional REST API for programmatic device management (actix-web)
- **rootAVD Integration** — One-click root/unroot via rootAVD ramdisk patching

## Screenshots

<!-- TODO: add screenshots -->

## Prerequisites

- **Rust** (stable, edition 2021) — [rustup.rs](https://rustup.rs)
- **Node.js** 18+ and npm — [nodejs.org](https://nodejs.org)
- **Android SDK** — installed and configured (set `ANDROID_HOME` or configure in-app)
  - Required SDK packages:
    - `platform-tools` (for adb)
    - `cmdline-tools;latest` (for avdmanager, sdkmanager)
    - `emulator`
    - At least one system image (e.g., `system-images;android-34;google_apis;x86_64`)
- **Git** — for rootAVD integration (optional)

## Install

```bash
# Clone the repository
git clone https://github.com/nousresearch/enmulator.git
cd enmulator

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Usage

1. Launch Enmulator. The app will auto-detect your Android SDK from `ANDROID_HOME` or standard paths.
2. Click **+ New Device** to create an AVD. Choose a name, device profile, and API level.
3. Click **▶ Start** to launch the emulator.
4. Use the **Quick Actions** toolbar at the bottom for common tasks:
   - Shell, Profile, Record, GPS, Log, Clipboard, Files, Snapshots
5. Toggle **☐ Select** to enter batch mode — select multiple devices and perform operations on all of them.
6. For root access, click **⬇ rootAVD** to clone/update the rootAVD repository, then click **🔒 Root** on a stopped device.

### rootAVD

rootAVD patches the emulator's `ramdisk.img` to enable root access. The workflow:

1. Click **⬇ rootAVD** in the header to clone https://gitlab.com/newbit/rootAVD
2. Ensure the device is **stopped**
3. Click **🔒 Root** in the device's Quick Actions
4. The ramdisk is backed up to `ramdisk.img.bak`, then patched
5. Start the device — it will now have root access
6. To unroot, stop the device and click **🔓 Unroot** — the original ramdisk is restored

## API

When the API server is started (via the `start_api_server` command or UI), the following endpoints are available:

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/devices` | List all devices |
| `POST` | `/api/devices` | Create a new device |
| `POST` | `/api/devices/{id}/start` | Start a device (headless) |
| `POST` | `/api/devices/{id}/stop` | Stop a device |
| `DELETE` | `/api/devices/{id}` | Delete a device |
| `POST` | `/api/devices/{id}/clone` | Clone a device |
| `POST` | `/api/devices/{id}/adb` | Run ADB shell command |
| `GET` | `/api/devices/{id}/logcat` | Stream logcat via SSE |
| `GET` | `/api/profiles` | List fingerprint profiles |

## Project Structure

```
enmulator/
├── src/                          # React frontend
│   ├── App.tsx                    # Main application component
│   ├── main.tsx                   # React entry point
│   └── components/
│       ├── CreateWizard.tsx       # New device creation modal
│       ├── DeviceCard.tsx         # Per-device card with controls
│       ├── FileExplorer.tsx       # Device file browser
│       ├── QuickActions.tsx       # Per-device quick action buttons
│       └── SnapshotPanel.tsx      # Snapshot management UI
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                # Tauri commands & app entry
│       ├── sdk.rs                 # Android SDK detection & paths
│       ├── config.rs              # Configuration persistence
│       ├── device.rs              # Device struct & store
│       ├── avd_manager.rs         # AVD creation & listing
│       ├── emulator.rs            # Emulator process management
│       ├── adb_bridge.rs          # ADB operations & snapshots
│       ├── fingerprint.rs         # Fingerprint profiles
│       ├── extras.rs              # Recording, clipboard, GPS, logcat
│       ├── set_proxy.rs           # Per-device proxy config
│       ├── api_server.rs          # REST API (actix-web)
│       └── rootavd.rs             # rootAVD integration
├── package.json
├── vite.config.ts
└── config.json                   # Runtime configuration
```

## License

MIT
