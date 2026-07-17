# Scanapp

A free, local-first desktop LAN scanner built with **Tauri 2**, **Rust**, and **React**. Discover devices on your network and see IP, hostname, MAC, vendor, and online status — no account, no subscription.

## Features (MVP)

- Detect your active IPv4 interface, subnet, and gateway
- Concurrent host discovery (UDP ARP-seed + TCP connect probe)
- MAC addresses from the OS neighbor/ARP table
- Vendor lookup from a bundled IEEE OUI database
- Reverse DNS hostnames
- Nickname persistence across scans
- Live scan progress in the UI

## Prerequisites

### All platforms

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) 1.85+ (stable)

### Linux

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libgtk-3-dev
```

See also: [Tauri Linux prerequisites](https://tauri.app/start/prerequisites/).

### macOS

- Xcode Command Line Tools: `xcode-select --install`

### Windows

- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- WebView2 (usually preinstalled on Windows 10/11)

## Develop

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

Installers appear under `src-tauri/target/release/bundle/`.

## How scanning works

1. Detect the primary private IPv4 interface and CIDR (subnets larger than `/22` / 1024 hosts are rejected for safety).
2. Probe each host with a UDP packet (to populate ARP) and short TCP connects to common ports.
3. Read the OS ARP/neighbor table for MAC addresses.
4. Enrich with reverse DNS and OUI vendor names.
5. Keep previously seen devices marked offline if they disappear.

**Permissions:** Most home networks work without elevation. Raw ICMP is not required. On some networks, firewalled hosts may only appear after ARP seeding — results depend on OS neighbor-table visibility.

## Data storage

Device nicknames and last-known devices are stored locally as JSON in the app data directory (platform-specific via Tauri path resolver). Nothing is uploaded.

## Project layout

```
src/                 React UI
src-tauri/           Rust / Tauri backend
  src/network/       Discovery, ARP, DNS, OUI
  resources/oui.txt  Bundled vendor database
```

## Roadmap (not in this release)

- Ping tool
- TCP port scan
- Wake-on-LAN
- Mobile (Tauri)
