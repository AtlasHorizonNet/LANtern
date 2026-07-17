# LANtern

[![CI](https://github.com/AtlasHorizonNet/LANtern/actions/workflows/ci.yml/badge.svg)](https://github.com/AtlasHorizonNet/LANtern/actions/workflows/ci.yml)
[![Release](https://github.com/AtlasHorizonNet/LANtern/actions/workflows/release.yml/badge.svg)](https://github.com/AtlasHorizonNet/LANtern/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/AtlasHorizonNet/LANtern?sort=semver)](https://github.com/AtlasHorizonNet/LANtern/releases/latest)

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

## Tests

Rust backend tests (recommended; this is where scanning logic lives):

```bash
cd src-tauri
cargo test
```

These cover OUI lookup, MAC normalization, ARP table parsers, subnet helpers, and local device-store persistence. A few smoke checks also exercise live interface/neighbor APIs when available.

Frontend UI tests are not included yet — most of the product risk is in the Rust network layer.

## Continuous integration & releases

Three GitHub Actions workflows cover Linux, macOS, and Windows:

- **CI** (`.github/workflows/ci.yml`) — runs on every push to `main` and on pull requests. It type-checks/builds the frontend, checks Rust formatting (`cargo fmt`), lints with `cargo clippy -D warnings`, and runs the Rust test suite on all three platforms.
- **Release** (`.github/workflows/release.yml`) — runs on every push to `main`. It uses [release-please](https://github.com/googleapis/release-please) to manage semantic versioning, then builds and attaches installers to the GitHub Release.
- **Build (artifacts)** (`.github/workflows/build.yml`) — a manual (`workflow_dispatch`) workflow for building installers for all four targets and uploading them as workflow artifacts, handy for testing a bundle without cutting a release.

### Releases & semantic versioning

Versioning is automated with [Conventional Commits](https://www.conventionalcommits.org/) via release-please:

1. Land commits on `main` using Conventional Commit messages:
   - `fix: ...` → patch bump (e.g. `0.1.0` → `0.1.1`)
   - `feat: ...` → minor bump (e.g. `0.1.0` → `0.2.0`)
   - `feat!: ...` or a `BREAKING CHANGE:` footer → major bump (e.g. `0.1.0` → `1.0.0`)
2. release-please opens/maintains a **release PR** that bumps the version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, and updates `CHANGELOG.md`.
3. Merging that release PR creates the git tag and GitHub Release, and the same workflow builds installers for Linux, Windows, macOS (Apple Silicon), and macOS (Intel) and **uploads them as release assets**.

The tracked version lives in `.release-please-manifest.json`; behavior is configured in `release-please-config.json`.

> **Repo setting required for the automated flow:** release-please opens the release PR, so enable **Settings → Actions → General → Workflow permissions → "Allow GitHub Actions to create and approve pull requests."** Without it, release-please can bump versions but cannot open the release PR.

> Note: because the release PR is opened with the default `GITHUB_TOKEN`, CI checks do not run on it. To run CI on release PRs, supply a personal access token (with `contents: write` and `pull-requests: write`) as the `token:` input in `release.yml`.

### Cutting a release by tag (manual path)

You can also release without the release-please PR by pushing a semantic-version tag. This builds all four targets and attaches the installers to a matching GitHub Release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Roadmap (tracked as GitHub issues)

- Ping tool
- TCP port scan
- Wake-on-LAN
- DHCP test tool
- Mobile (Tauri)

If these issues are not yet on the repo, create them with:

```bash
./scripts/create-roadmap-issues.sh
```
