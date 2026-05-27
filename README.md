# ADB Bar

A cross-platform system tray app for managing ADB devices. Built with Tauri (Rust + Svelte).

## Features

- System tray icon with popup window
- Connect/disconnect ADB devices with one click
- Auto-scan local network for ADB devices (port 5555)
- Manual device addition
- Quick actions: Shell, Scrcpy mirror, Screenshot, Install APK
- Configurable ADB path with auto-detection
- Scrcpy auto-detection and one-click installation
- Network change detection with auto-refresh
- Device list persistence across restarts

## Platforms

- macOS (Apple Silicon / Intel)
- Windows

## Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/)
- [adb](https://developer.android.com/tools/releases/platform-tools) in PATH (or configure path in Settings)

## Development

```bash
# Install dependencies
npm install

# Run in development mode
npx tauri dev

# Build for production
npx tauri build
```

## Tech Stack

- **Backend**: Rust (Tauri v2)
- **Frontend**: Svelte 5 + TypeScript + Vite
- **Networking**: Pure Rust async TCP scanner (tokio)

## License

[MIT](LICENSE)
