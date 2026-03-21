# Installation Guide

## System Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| **macOS** | 12 Monterey+ (Apple Silicon or Intel) | 13 Ventura+ (Apple Silicon) |
| **Windows** | 10 x64 | 11 x64 |
| **Linux** | Ubuntu 22.04+ x64 | Ubuntu 24.04+ x64 |
| **RAM** | 8 GB | 16 GB |
| **Disk** | 6 GB free (for models) | 10 GB free |
| **Node.js** | 20+ | 22 LTS |
| **Rust** | 1.75+ (stable) | Latest stable |

## Pre-Built Releases

The easiest way to install Justice AI is to download a pre-built release:

1. Go to [GitHub Releases](https://github.com/lastbaa/CS-370-Justice-AI-Project/releases)
2. Download the installer for your platform:
   - **macOS**: `Justice.AI-x.x.x-arm64.dmg` (Apple Silicon) or `Justice.AI-x.x.x-x64.dmg` (Intel)
   - **Windows**: `Justice.AI-x.x.x-x64-setup.exe`
   - **Linux**: `Justice.AI-x.x.x-amd64.deb` or `.AppImage`
3. Install and launch. Models will be downloaded on first run.

## Building from Source

### Prerequisites (All Platforms)

- **Node.js 20+**: [nodejs.org](https://nodejs.org/)
- **Rust toolchain**: [rustup.rs](https://rustup.rs/)

### macOS

Install Xcode command line tools and Rust:

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Or via Homebrew:

```bash
brew install rust node
```

### Windows

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload
2. Install Rust via [rustup](https://rustup.rs/) (choose the default options)
3. Install [Node.js 20+](https://nodejs.org/)
4. Restart your terminal after installation

### Linux (Ubuntu/Debian)

Install system dependencies required by Tauri:

```bash
sudo apt update
sudo apt install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libssl-dev \
  build-essential \
  curl \
  wget
```

Install Rust and Node.js:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
```

### Build & Run

Clone the repository and install dependencies:

```bash
git clone https://github.com/lastbaa/CS-370-Justice-AI-Project.git
cd CS-370-Justice-AI-Project
npm install
```

**Development mode** (hot reload):

```bash
npm run app
```

**Production build**:

```bash
npm run build:app
```

The built installer will be in `app/src-tauri/target/release/bundle/`.

## Troubleshooting

### Rust compilation errors
- Run `rustup update` to ensure you have the latest stable toolchain.
- On Linux, double-check all system libraries from the list above are installed.

### Node.js version mismatch
- Run `node --version` to verify you have 20+. Use [nvm](https://github.com/nvm-sh/nvm) to manage versions if needed.

### macOS: "app is damaged" or "unidentified developer"
- Right-click the app and select **Open**, then confirm in the dialog.
- Or go to **System Preferences > Security & Privacy > General** and click **Open Anyway**.

### Windows: WebView2 missing
- Tauri requires Microsoft Edge WebView2. It's included in Windows 11 and most Windows 10 installs. If missing, download it from [Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

### Linux: WebKitGTK not found
- Ensure `libwebkit2gtk-4.1-dev` is installed. On older distributions, you may need to add a PPA or build from source.
