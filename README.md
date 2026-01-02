
# AntigravityEQ

**AntigravityEQ** is a modern, high-performance GUI for **EqualizerAPO**, built with **Tauri v2** and **Next.js**. It provides a sleek, dark-themed parametric equalizer interface that syncs in real-time with your system audio.

![AntigravityEQ UI](https://via.placeholder.com/800x450?text=AntigravityEQ+Screenshot)

## Features

- 🎚️ **Parametric EQ**: Add unlimited peaking, low-shelf, and high-shelf filters.
- ⚡ **Real-Time Sync**: Changes update EqualizerAPO instantly (~10ms latency).
- ⚠️ **Peak Gain Safety**: Built-in meter warns about potential clipping (>0dB).
- 📊 **Audio Status Display**: Real-time output device, format (bits/Hz), and volume peak meter.
- 🎛️ **Precise Control**: Text inputs for Preamp and Bands for exact value tuning.
- 💾 **Persistence**: Auto-saves your UI state (sliders, profiles) so you never lose work.
- 📂 **Profile Management**: Save and load unlimited JSON profiles.
- 🔄 **Universal Import/Export**: Import from JSON or EqualizerAPO `.txt` usage files.
- 🎨 **Modern Design**: Built with Shadcn/UI and TailwindCSS.
- 🛡️ **Robust**: Auto-fixes file permissions for smooth operation.

---

## 🚀 Setup Guide

### 1. Prerequisites

Before running AntigravityEQ, ensure you have the following:

*   **EqualizerAPO**: Installed and active on your audio device. [Download Here](https://sourceforge.net/projects/equalizerapo/).
*   **Node.js** (v18+) or **Bun** (recommended).
*   **Rust & Cargo**: Required for the Tauri backend. [Install Rust](https://rustup.rs/).
*   **Microsoft VS C++ Build Tools**: Required for compiling Rust on Windows.

### 2. Installation

Clone the repository and install dependencies:

```bash
# Clone
git clone https://github.com/your-username/antigravity-eq.git
cd antigravity-eq

# Install dependencies (using Bun)
bun install
# OR using NPM
npm install
```

### 3. Running in Development Mode

To start the app with hot-reloading:

```bash
# Using Bun (Recommended)
bun run tauri dev

# OR using NPM
npm run tauri dev
```

> **Note**: If you want to configure the app to write directly to `C:\Program Files\EqualizerAPO\config\`, you must run your terminal as **Administrator**.

### 4. Release / Building for Production

To create an optimized `.exe` installer (without dev tools):

```bash
# 1. Clean previous builds (optional but recommended)
rm -rf src-tauri/target/release

# 2. Build the installer
bun run tauri build
```

Your final installer will be ready at:
`src-tauri/target/release/bundle/nsis/AntigravityEQ_x.x.x_x64-setup.exe`

Double-click this file to install AntigravityEQ on your machine.
**Note:** The release version shares the same configuration as your dev version, so your profiles and settings will be preserved.

---

## 🛠️ Usage & Configuration

### Connect to EqualizerAPO

1.  Open **AntigravityEQ**.
2.  Go to **File > Set Live Config Path...**.
3.  Choose where you want the active config file to live.
    *   **Recommended**: Default (`Documents/AntigravityEQ/live_config.txt`).
    *   **Advanced**: Directly select `C:\Program Files\EqualizerAPO\config\config.txt`.
4.  **Important**: If using the default path, open **EqualizerAPO Configuration Editor** and add this line to your `config.txt`:
    ```
    Include: C:\Users\YOUR_USERNAME\Documents\AntigravityEQ\live_config.txt
    ```

### Troubleshooting

**"Write Access Denied"**
*   If writing to Program Files, run the app as Administrator.
*   AntigravityEQ will automatically attempt to fix file permissions if it encounters an error. Check the terminal logs in Dev mode for details.

**"Audio Not Changing"**
*   Ensure EqualizerAPO is installed for your specific audio device (Configurator.exe).
*   Ensure the `Include:` line in EqualizerAPO points exactly to the path shown in AntigravityEQ settings.

---

## 📊 Audio Status Display

The status bar at the bottom of the window shows real-time audio information:

| Field | Description |
|-------|-------------|
| **Output Device** | The active Windows audio output device name |
| **Format** | Bit depth and sample rate (e.g., "32-bit float / 48 kHz") |
| **Peak Meter** | Real-time output level in dBFS with visual bar |

### Peak Meter Details

- **Signal Source**: Measured **post-Windows-mixer** via WASAPI loopback capture
- **Update Rate**: ~30 FPS for smooth visualization
- **Peak Hold**: 1 second before decay
- **Color Coding**:
  - 🟢 **Green**: Safe levels (< -6 dBFS)
  - 🟡 **Yellow**: Approaching clipping (-6 to -0.5 dBFS)
  - 🔴 **Red**: Clipping detected (> -0.5 dBFS)

> **Note**: The peak meter shows system-wide output levels. If other applications are playing audio, their signal will be included in the measurement. This reflects the final mixed output, not the EQ-processed signal specifically.

---

## Tech Stack

*   **Frontend**: Next.js 16.1 (App Router), React 19, TailwindCSS v4, Shadcn/UI
*   **Backend**: Rust (Tauri v2)
*   **State**: LocalStorage (Persistence), FileSystem (Profiles)

