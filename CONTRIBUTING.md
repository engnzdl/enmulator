# Contributing to Enmulator

Thanks for your interest in contributing. Enmulator is a cross-platform Android emulator manager built with Tauri 2 + Rust + React, licensed under GPLv3.

## Getting Started

```bash
git clone https://github.com/engnzdl/enmulator.git
cd enmulator
npm install
```

### Development

You need two terminals (or run Vite separately):

```bash
# Terminal 1: Frontend dev server
npx vite --port 1420

# Terminal 2: Tauri dev server
export PATH="$HOME/.cargo/bin:$PATH"
npm run tauri dev
```

### Prerequisites

- [Rust](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) (18+)
- [Android SDK Command-line Tools](https://developer.android.com/studio#command-line-tools-only)
  - At least one system image (downloadable from within the app)
- **Linux:** `libwebkit2gtk-4.1`, `libgtk-3`, KVM recommended
- **Windows:** WebView2 (pre-installed on Windows 11)

## Project Structure

```
enmulator/
├── src/                    # React frontend (TypeScript)
│   ├── App.tsx             # Main layout, state management
│   ├── App.css             # Global styles (dark theme)
│   └── components/         # React components
├── src-tauri/              # Rust backend (Tauri)
│   ├── src/
│   │   ├── main.rs         # Tauri command handlers
│   │   ├── emulator.rs     # Emulator process lifecycle
│   │   ├── sdk.rs          # Android SDK detection + tools
│   │   ├── adb_bridge.rs   # ADB shell, setprop, push/pull
│   │   ├── fingerprint.rs  # Profile management + identity apply
│   │   ├── device.rs       # Device struct + persistent store
│   │   ├── config.rs       # App configuration
│   │   ├── paths.rs        # Cross-platform config/data dirs
│   │   ├── avd_manager.rs  # AVD creation + config.ini overrides
│   │   ├── extras.rs       # Recording, GPS, clipboard, logcat
│   │   ├── bypass.rs       # Emulator detection bypass
│   │   ├── set_proxy.rs    # Per-device HTTP proxy
│   │   └── api_server.rs   # REST API (Actix-web)
│   ├── capabilities/       # Tauri v2 permissions
│   └── tauri.conf.json     # Tauri configuration
├── profiles/               # 30 fingerprint profiles (JSON)
├── package.json
├── vite.config.ts
└── README.md
```

## Before Submitting

```bash
# TypeScript check (must pass)
npx tsc --noEmit

# Rust build (must pass)
export PATH="$HOME/.cargo/bin:$PATH"
cd src-tauri && cargo build
```

Both must exit with zero errors. Warnings are acceptable but try to minimize them.

## Commit Guidelines

- Use present tense, imperative mood: `fix: ...`, `feat: ...`, `docs: ...`, `cleanup: ...`
- Keep commits focused — one logical change per commit
- Reference issues when applicable: `fix: #42 — handle port conflict`

## Pull Requests

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Ensure `npx tsc --noEmit` and `cargo build` both pass
4. Update documentation if you change behavior
5. Open a PR with a clear description

## Code Style

- **Rust:** standard `rustfmt` conventions (run `cargo fmt`)
- **TypeScript/React:** 2-space indent, single quotes, no semicolons
- **CSS:** kebab-case class names, dark theme variables from `App.css`
- No console.log in production paths (development debugging is fine)

## Adding Fingerprint Profiles

Profiles are JSON files in `profiles/`. To add a new device:

1. Create `profiles/brand_model.json` following the existing format
2. Include realistic: brand, model, manufacturer, device codename, build fingerprint, DPI, resolution
3. Generate a valid Luhn-checksum IMEI (15 digits starting with 35 for generic devices)
4. Use region-appropriate SIM operator codes (MCC+MNC)

## Questions?

Open an issue or start a discussion on GitHub.

---

GPLv3 © engnzdl
