# Changelog

All notable changes to Enmulator.

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
- Cross-platform path handling (no shell assumptions, no hardcoded `/tmp`)
- GPLv3 license

### Technical
- 13 Rust modules: main, emulator, sdk, adb_bridge, fingerprint, device, config, paths, avd_manager, extras, bypass, set_proxy, api_server
- 7 React components: App, DeviceCard, CreateWizard, QuickActions, ProxyCard, FileExplorer, SettingsPanel
- ~25 Tauri IPC commands
- Persistent device store (JSON)
- System image download with real-time progress streaming
- `adb root` support on Google APIs images
- Config.ini resolution/DPI overrides per profile
