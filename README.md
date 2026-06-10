# ADB Bar

[English](#english) | [中文](#中文)

---

<a id="english"></a>

A cross-platform system tray app for managing ADB devices. Built with Tauri (Rust + Svelte).

## Features

- System tray icon with popup window
- Connect/disconnect ADB devices with one click
- Auto-scan local network for ADB devices (port 5555)
- Manual device addition
- Quick actions: Shell, Scrcpy mirror, Screenshot, Install APK
- Configurable ADB path with auto-detection
- Scrcpy auto-detection and one-click installation
- ADB quick tools: restart ADB server, enable TCP/IP mode
- Smart refresh: reconnects dropped connections on manual refresh
- Device list persistence across restarts
- Bilingual UI: English & Chinese (auto-detect system language)
- System notifications for tray actions (connect, restart ADB, TCP/IP)

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

---

<a id="中文"></a>

跨平台 ADB 设备管理工具，常驻系统托盘。基于 Tauri (Rust + Svelte) 构建。

## 功能

- 系统托盘图标，点击弹出窗口
- 一键连接/断开 ADB 设备
- 自动扫描局域网中的 ADB 设备（端口 5555）
- 手动添加设备
- 快捷操作：Shell 终端、Scrcpy 投屏、截屏、安装 APK
- 可配置 ADB 路径，支持自动检测
- Scrcpy 自动检测与一键安装（macOS: Homebrew，Windows: 自动下载）
- ADB 快捷工具：重启 ADB 服务、一键开启 TCP/IP 模式
- 智能刷新：手动刷新时自动重连已断开的设备
- 设备列表持久化，重启后保留
- 中英双语界面（自动检测系统语言）
- 托盘操作系统通知（连接、重启 ADB、TCP/IP）

## 支持平台

- macOS (Apple Silicon / Intel)
- Windows

## 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/)
- [adb](https://developer.android.com/tools/releases/platform-tools)（可在设置中配置路径）

## 开发

```bash
# 安装依赖
npm install

# 开发模式运行
npx tauri dev

# 生产构建
npx tauri build
```

## 技术栈

- **后端**: Rust (Tauri v2)
- **前端**: Svelte 5 + TypeScript + Vite
- **网络扫描**: 纯 Rust 异步 TCP 扫描器 (tokio)

## License

[MIT](LICENSE)
