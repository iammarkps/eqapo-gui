# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

EQAPO GUI is a Tauri v2 desktop application providing a modern GUI for EqualizerAPO (Windows audio equalizer). It features real-time parametric EQ editing, peak metering via WASAPI, profile management, and A/B blind testing.

## Build Commands

```bash
bun run tauri dev     # Run desktop app in dev mode (hot reload)
bun run tauri build   # Build Windows installer (.exe/.msi)
bun run dev           # Next.js dev server only (http://localhost:3000)
bun run build         # Next.js production build
bun run test          # Run Vitest unit tests
```

## Architecture

**Frontend (Next.js 16 + React 19)**
- `app/page.tsx` - Main UI, orchestrates all components via `useEqualizer` hook
- `lib/use-equalizer.ts` - Core state management (bands, preamp, profiles, EAPO sync)
- `lib/tauri.ts` - IPC wrapper functions calling Rust commands
- `lib/audio-math.ts` - Biquad filter frequency response calculations
- `components/` - UI components (eq-graph, band-editor, peak-meter, profile-selector)

**Backend (Rust/Tauri)**
- `src-tauri/src/lib.rs` - Tauri command handlers (entry point for IPC)
- `src-tauri/src/commands.rs` - Command implementations
- `src-tauri/src/profile.rs` - Profile & settings file I/O
- `src-tauri/src/audio_monitor.rs` - WASAPI loopback capture for peak metering
- `src-tauri/src/ab_test.rs` - A/B testing session management

**Data Flow**
```
React State → Tauri IPC → Rust Commands → Files
                                         ├── EqualizerAPO config (live_config.txt)
                                         └── App data (Documents/EQAPO GUI/)
                                             ├── settings.json
                                             └── profiles/*.json
```

## Key Concepts

**EQ Bands**: Three filter types (Peaking, Low Shelf, High Shelf) with frequency, gain, and Q parameters. Converted to EAPO format: `Filter: ON {TYPE} Fc {FREQ} Hz Gain {GAIN} dB Q {Q}`

**Profile System**: Named EQ configurations stored as JSON. Supports import from EqualizerAPO `.txt` files.

**Audio Monitoring**: Windows-only WASAPI loopback capture provides device info and real-time peak levels.

## Tech Stack

- Tauri v2, Next.js 16 (static export), React 19, TailwindCSS v4
- Shadcn/UI (Radix primitives), Lucide icons, next-themes
- Rust with Windows crate for WASAPI, Serde for JSON
- Vitest for testing

## Version Management

Versions must stay in sync across three files:
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

See `docs/VERSIONING.md` for release workflow details.
