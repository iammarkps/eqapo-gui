//! EQAPO GUI - Tauri Backend Library
//!
//! This crate provides the Rust backend for the EQAPO GUI desktop application,
//! a modern graphical interface for [EqualizerAPO], the open-source system-wide
//! audio equalizer for Windows.
//!
//! [EqualizerAPO]: https://sourceforge.net/projects/equalizerapo/
//!
//! # Architecture Overview
//!
//! The backend is built on [Tauri v2] and provides:
//!
//! - **Profile Management**: Save, load, and switch between EQ configurations
//! - **Real-time EQ Control**: Apply parametric EQ bands and preamp to EqualizerAPO
//! - **System Tray Integration**: Quick profile switching from the Windows tray
//! - **A/B Testing**: Blind listening tests to evaluate EQ differences
//! - **Peak Metering**: Real-time audio level monitoring via WASAPI (Windows only)
//!
//! [Tauri v2]: https://v2.tauri.app/
//!
//! # Module Structure
//!
//! | Module          | Description                                    |
//! |-----------------|------------------------------------------------|
//! | [`types`]       | Core data types (FilterType, EqProfile, etc.)  |
//! | [`profile`]     | Profile and settings file I/O                  |
//! | [`commands`]    | A/B testing Tauri command handlers             |
//! | [`tray`]        | System tray menu and event handling            |
//! | [`ab_test`]     | A/B/X blind testing session logic              |
//! | [`audio_monitor`]| WASAPI peak metering (Windows only)           |
//!
//! # Data Flow
//!
//! ```text
//! ┌─────────────┐     IPC      ┌─────────────────┐     File I/O
//! │  Frontend   │ ◄──────────► │  Tauri Commands │ ◄──────────►
//! │  (React)    │              │  (This Crate)   │
//! └─────────────┘              └────────┬────────┘
//!                                       │
//!                                       ▼
//!                              ┌─────────────────┐
//!                              │  EqualizerAPO   │
//!                              │  Config File    │
//!                              └─────────────────┘
//! ```
//!
//! # Public API
//!
//! This crate exports the following types for use in tests and external code:
//!
//! - [`FilterType`] - Enum of supported EQ filter types
//! - [`ParametricBand`] - Single EQ band configuration
//! - [`EqProfile`] - Complete EQ profile with name, preamp, and bands
//! - [`AppSettings`] - Persistent application settings
//! - [`AppState`] - Runtime state managed by Tauri
//!
//! # Entry Point
//!
//! The [`run()`] function initializes and starts the Tauri application:
//!
//! ```ignore
//! fn main() {
//!     eqapo_gui_lib::run();
//! }
//! ```
//!
//! # Platform Support
//!
//! While the core functionality works on any platform Tauri supports, the following
//! features are Windows-only:
//!
//! - Audio output device information
//! - Peak meter monitoring (WASAPI loopback)
//! - EqualizerAPO config file permission handling

use parking_lot::Mutex;
use tauri::WindowEvent;

// =============================================================================
// Module Declarations
// =============================================================================

/// A/B and blind listening test session management.
mod ab_test;

/// Tauri command handlers for A/B testing.
mod commands;

/// Profile and settings file I/O operations.
mod profile;

/// System tray icon and menu handling.
mod tray;

/// Core data types shared across modules.
mod types;

/// Windows audio monitoring via WASAPI (Windows only).
#[cfg(windows)]
mod audio_monitor;

// =============================================================================
// Re-exports
// =============================================================================

// Internal use
use profile::load_settings;
use tray::setup_tray;

// Public API - these types are used by tests and could be used by external code
pub use types::{AppSettings, AppState, EqProfile, FilterType, ParametricBand};

// Re-export commands for Tauri handler
use commands::{
    apply_ab_option, finish_ab_session, get_ab_state, record_ab_answer, start_ab_session,
    update_ab_trim,
};
use profile::{
    apply_profile, delete_profile, get_current_profile, get_settings, list_profiles, load_profile,
    save_profile, set_current_profile, update_settings,
};
use tray::refresh_tray_menu;

#[cfg(windows)]
use audio_monitor::{AudioMonitor, AudioOutputInfo, PeakMeterUpdate};
#[cfg(windows)]
use std::sync::Arc;
#[cfg(windows)]
use tauri::{AppHandle, Emitter};

// =============================================================================
// Audio Monitor Commands (Windows-only)
// =============================================================================
//
// These commands provide real-time audio monitoring capabilities via Windows
// WASAPI loopback capture. On non-Windows platforms, they return errors.

/// Retrieves information about the current default audio output device.
///
/// Queries the Windows audio subsystem for the default playback device and
/// returns its properties including name, sample rate, bit depth, and channel
/// configuration.
///
/// # Returns
///
/// On success, returns [`AudioOutputInfo`] containing:
/// - `device_name`: Human-readable device name (e.g., "Speakers (Realtek Audio)")
/// - `device_id`: Windows device identifier
/// - `sample_rate`: Current sample rate in Hz (e.g., 48000)
/// - `bit_depth`: Bits per sample (e.g., 16, 24, 32)
/// - `channel_count`: Number of audio channels (e.g., 2 for stereo)
/// - `format_tag`: Audio format ("PCM" or "IEEE Float")
///
/// # Errors
///
/// Returns an error string if:
/// - COM initialization fails
/// - No default audio device is available
/// - Device properties cannot be queried
///
/// # Platform
///
/// **Windows only.** On other platforms, returns:
/// `Err("Audio monitoring is only available on Windows")`
///
/// # Example (Frontend)
///
/// ```javascript
/// const info = await invoke('get_audio_output_info');
/// console.log(`${info.device_name}: ${info.sample_rate}Hz, ${info.bit_depth}-bit`);
/// ```
#[cfg(windows)]
#[tauri::command]
fn get_audio_output_info(state: tauri::State<AppState>) -> Result<AudioOutputInfo, String> {
    state.audio_monitor.get_audio_output_info()
}

/// Stub for non-Windows platforms.
#[cfg(not(windows))]
#[tauri::command]
fn get_audio_output_info() -> Result<(), String> {
    Err("Audio monitoring is only available on Windows".to_string())
}

/// Starts continuous real-time peak meter monitoring.
///
/// Initiates audio capture on a background thread that samples the system's
/// audio output and calculates peak levels. Results are emitted to the frontend
/// via Tauri events at approximately 30 FPS.
///
/// # Event: `peak_meter_update`
///
/// Emitted continuously while monitoring is active:
///
/// ```json
/// {
///   "peak_db": -12.5,
///   "peak_linear": 0.237,
///   "timestamp": 1704067200000
/// }
/// ```
///
/// # Arguments
///
/// * `state` - Application state containing the audio monitor
/// * `app` - Tauri app handle for emitting events to the frontend
///
/// # Errors
///
/// Returns an error if:
/// - Monitoring is already active (safe to call, just returns Ok)
/// - WASAPI loopback capture cannot be initialized
/// - The default audio device is not available
///
/// # Resource Usage
///
/// The monitoring thread consumes minimal CPU (~1-2%) but should be stopped
/// when not needed using [`stop_peak_meter`]. The monitor automatically
/// reconnects if the audio device changes.
///
/// # Platform
///
/// **Windows only.** On other platforms, returns an error.
#[cfg(windows)]
#[tauri::command]
fn start_peak_meter(state: tauri::State<AppState>, app: AppHandle) -> Result<(), String> {
    let app_handle = app.clone();
    state.audio_monitor.start_peak_monitoring(move |update| {
        let _ = app_handle.emit("peak_meter_update", update);
    })
}

/// Stub for non-Windows platforms.
#[cfg(not(windows))]
#[tauri::command]
fn start_peak_meter() -> Result<(), String> {
    Err("Audio monitoring is only available on Windows".to_string())
}

/// Stops the peak meter monitoring background thread.
///
/// Signals the monitoring thread to stop and waits for it to exit cleanly.
/// This releases audio capture resources and stops event emission.
///
/// Call this when:
/// - The peak meter UI is hidden or closed
/// - The application is minimizing to save resources
/// - Switching to a different feature that doesn't need metering
///
/// # Thread Safety
///
/// Safe to call even if monitoring is not active (no-op in that case).
/// Safe to call from any thread.
///
/// # Platform
///
/// **Windows only.** On other platforms, this is a no-op.
#[cfg(windows)]
#[tauri::command]
fn stop_peak_meter(state: tauri::State<AppState>) {
    state.audio_monitor.stop_peak_monitoring();
}

/// Stub for non-Windows platforms.
#[cfg(not(windows))]
#[tauri::command]
fn stop_peak_meter() {}

/// Returns the current peak level without starting continuous monitoring.
///
/// Provides a one-shot reading of the most recent peak value. Useful for:
/// - Low-frequency polling when real-time updates aren't needed
/// - Checking levels before starting a recording or test
/// - Debugging audio routing issues
///
/// # Returns
///
/// A [`PeakMeterUpdate`] containing:
/// - `peak_db`: Peak level in decibels (0 dB = full scale, negative = below)
/// - `peak_linear`: Peak level as a linear value (0.0 to 1.0+)
/// - `timestamp`: Unix timestamp in milliseconds
///
/// If monitoring is not active, returns the last captured value or silence (-100 dB).
///
/// # Note
///
/// For continuous monitoring, prefer [`start_peak_meter`] which is more efficient
/// than polling this command repeatedly.
///
/// # Platform
///
/// **Windows only.** On other platforms, returns an error.
#[cfg(windows)]
#[tauri::command]
fn get_current_peak(state: tauri::State<AppState>) -> PeakMeterUpdate {
    state.audio_monitor.get_current_peak()
}

/// Stub for non-Windows platforms.
#[cfg(not(windows))]
#[tauri::command]
fn get_current_peak() -> Result<(), String> {
    Err("Audio monitoring is only available on Windows".to_string())
}

// =============================================================================
// Application Entry Point
// =============================================================================

/// Initializes and runs the EQAPO GUI Tauri application.
///
/// This is the main entry point that sets up the entire application:
///
/// 1. **Settings Loading**: Loads saved settings from `settings.json`, or uses defaults
/// 2. **Plugin Registration**: Initializes Tauri plugins for dialogs, file access, etc.
/// 3. **State Management**: Creates the [`AppState`] with settings, A/B session, and audio monitor
/// 4. **System Tray**: Sets up the tray icon with profile switching menu
/// 5. **Window Behavior**: Configures close-to-tray behavior
/// 6. **Command Registration**: Registers all IPC command handlers
///
/// # Behavior
///
/// - The application runs in the system tray
/// - Closing the window hides it instead of exiting (close-to-tray)
/// - Exit via the system tray "Quit" menu item
///
/// # Panics
///
/// Panics if the Tauri application fails to start. This typically only happens if:
/// - The window configuration is invalid
/// - System resources are exhausted
/// - Required plugins fail to initialize
///
/// # Example
///
/// ```ignore
/// fn main() {
///     eqapo_gui_lib::run();
/// }
/// ```
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = load_settings();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            settings: Mutex::new(settings),
            ab_session: Mutex::new(None),
            #[cfg(windows)]
            audio_monitor: Arc::new(AudioMonitor::new()),
        })
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Profile management
            list_profiles,
            load_profile,
            save_profile,
            apply_profile,
            delete_profile,
            get_current_profile,
            set_current_profile,
            get_settings,
            update_settings,
            refresh_tray_menu,
            // A/B testing
            start_ab_session,
            apply_ab_option,
            record_ab_answer,
            get_ab_state,
            finish_ab_session,
            update_ab_trim,
            // Audio monitoring
            get_audio_output_info,
            start_peak_meter,
            stop_peak_meter,
            get_current_peak
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use types::default_bands;

    // =========================================================================
    // FilterType Tests
    // =========================================================================

    #[test]
    fn filter_type_to_eapo_code_peaking() {
        assert_eq!(FilterType::Peaking.to_eapo_code(), "PK");
    }

    #[test]
    fn filter_type_to_eapo_code_lowshelf() {
        assert_eq!(FilterType::LowShelf.to_eapo_code(), "LSC");
    }

    #[test]
    fn filter_type_to_eapo_code_highshelf() {
        assert_eq!(FilterType::HighShelf.to_eapo_code(), "HSC");
    }

    // =========================================================================
    // ParametricBand Tests
    // =========================================================================

    #[test]
    fn parametric_band_to_eapo_line_peaking() {
        let band = ParametricBand {
            filter_type: FilterType::Peaking,
            frequency: 1000.0,
            gain: 6.0,
            q_factor: 1.41,
        };
        let line = band.to_eapo_line();
        assert_eq!(line, "Filter: ON PK Fc 1000 Hz Gain 6.0 dB Q 1.41");
    }

    #[test]
    fn parametric_band_to_eapo_line_lowshelf() {
        let band = ParametricBand {
            filter_type: FilterType::LowShelf,
            frequency: 100.0,
            gain: 3.5,
            q_factor: 0.71,
        };
        let line = band.to_eapo_line();
        assert_eq!(line, "Filter: ON LSC Fc 100 Hz Gain 3.5 dB Q 0.71");
    }

    #[test]
    fn parametric_band_to_eapo_line_highshelf() {
        let band = ParametricBand {
            filter_type: FilterType::HighShelf,
            frequency: 8000.0,
            gain: -2.0,
            q_factor: 0.707,
        };
        let line = band.to_eapo_line();
        assert_eq!(line, "Filter: ON HSC Fc 8000 Hz Gain -2.0 dB Q 0.71");
    }

    #[test]
    fn parametric_band_to_eapo_line_negative_gain() {
        let band = ParametricBand {
            filter_type: FilterType::Peaking,
            frequency: 500.0,
            gain: -3.5,
            q_factor: 2.0,
        };
        let line = band.to_eapo_line();
        assert!(line.contains("Gain -3.5 dB"));
    }

    #[test]
    fn parametric_band_to_eapo_line_frequency_truncated() {
        let band = ParametricBand {
            filter_type: FilterType::Peaking,
            frequency: 1234.567,
            gain: 0.0,
            q_factor: 1.0,
        };
        let line = band.to_eapo_line();
        assert!(line.contains("Fc 1234 Hz"));
    }

    // =========================================================================
    // AppSettings Tests
    // =========================================================================

    #[test]
    fn app_settings_default_bands_has_one_band() {
        let bands = default_bands();
        assert_eq!(bands.len(), 1);
    }

    #[test]
    fn app_settings_default_band_values() {
        let bands = default_bands();
        let band = &bands[0];
        assert_eq!(band.frequency, 1000.0);
        assert_eq!(band.gain, 0.0);
        assert_eq!(band.q_factor, 1.41);
    }

    #[test]
    fn app_settings_default_eq_enabled() {
        let settings = AppSettings::default();
        assert!(settings.eq_enabled);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn filter_type_serializes_lowercase() {
        let peaking = FilterType::Peaking;
        let json = serde_json::to_string(&peaking).unwrap();
        assert_eq!(json, "\"peaking\"");

        let lowshelf = FilterType::LowShelf;
        let json = serde_json::to_string(&lowshelf).unwrap();
        assert_eq!(json, "\"lowshelf\"");

        let highshelf = FilterType::HighShelf;
        let json = serde_json::to_string(&highshelf).unwrap();
        assert_eq!(json, "\"highshelf\"");
    }

    #[test]
    fn filter_type_deserializes_from_lowercase() {
        let peaking: FilterType = serde_json::from_str("\"peaking\"").unwrap();
        assert!(matches!(peaking, FilterType::Peaking));

        let lowshelf: FilterType = serde_json::from_str("\"lowshelf\"").unwrap();
        assert!(matches!(lowshelf, FilterType::LowShelf));

        let highshelf: FilterType = serde_json::from_str("\"highshelf\"").unwrap();
        assert!(matches!(highshelf, FilterType::HighShelf));
    }

    #[test]
    fn parametric_band_roundtrip_serialization() {
        let band = ParametricBand {
            filter_type: FilterType::Peaking,
            frequency: 1000.0,
            gain: 6.0,
            q_factor: 1.41,
        };

        let json = serde_json::to_string(&band).unwrap();
        let deserialized: ParametricBand = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.frequency, band.frequency);
        assert_eq!(deserialized.gain, band.gain);
        assert_eq!(deserialized.q_factor, band.q_factor);
    }

    #[test]
    fn eq_profile_serialization() {
        let profile = EqProfile {
            name: "Test Profile".to_string(),
            preamp: -3.5,
            bands: vec![ParametricBand {
                filter_type: FilterType::Peaking,
                frequency: 1000.0,
                gain: 6.0,
                q_factor: 1.41,
            }],
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"name\":\"Test Profile\""));
        assert!(json.contains("\"preamp\":-3.5"));
    }
}
