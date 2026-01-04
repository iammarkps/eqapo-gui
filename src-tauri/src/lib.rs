//! EQAPO GUI - Tauri backend library
//!
//! This module provides the core application logic for the EqualizerAPO GUI,
//! including profile management, system tray integration, and A/B testing.

use std::sync::Mutex;
use tauri::WindowEvent;

// Core modules
mod ab_test;
mod commands;
mod profile;
mod tray;
mod types;

#[cfg(windows)]
mod audio_monitor;

// Re-exports for internal use
use profile::load_settings;
use tray::setup_tray;
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

/// Returns information about the current default audio output device.
///
/// # Returns
///
/// Device name, sample rate, bit depth, and channel count.
///
/// # Platform
///
/// Available only on Windows. Returns an error on other platforms.
#[cfg(windows)]
#[tauri::command]
fn get_audio_output_info(state: tauri::State<AppState>) -> Result<AudioOutputInfo, String> {
    state.audio_monitor.get_audio_output_info()
}

#[cfg(not(windows))]
#[tauri::command]
fn get_audio_output_info() -> Result<(), String> {
    Err("Audio monitoring is only available on Windows".to_string())
}

/// Starts real-time peak meter monitoring.
///
/// Emits `peak_meter_update` events to the frontend with current
/// left/right channel peak levels at regular intervals.
///
/// # Arguments
///
/// * `state` - Tauri managed state containing the audio monitor
/// * `app` - App handle for emitting events
///
/// # Platform
///
/// Available only on Windows.
#[cfg(windows)]
#[tauri::command]
fn start_peak_meter(state: tauri::State<AppState>, app: AppHandle) -> Result<(), String> {
    let app_handle = app.clone();
    state.audio_monitor.start_peak_monitoring(move |update| {
        let _ = app_handle.emit("peak_meter_update", update);
    })
}

#[cfg(not(windows))]
#[tauri::command]
fn start_peak_meter() -> Result<(), String> {
    Err("Audio monitoring is only available on Windows".to_string())
}

/// Stops the peak meter monitoring thread.
///
/// Call this when the peak meter UI is hidden to conserve resources.
#[cfg(windows)]
#[tauri::command]
fn stop_peak_meter(state: tauri::State<AppState>) {
    state.audio_monitor.stop_peak_monitoring();
}

#[cfg(not(windows))]
#[tauri::command]
fn stop_peak_meter() {}

/// Returns the current peak levels without starting continuous monitoring.
///
/// Useful for one-time readings or low-frequency polling.
///
/// # Returns
///
/// Current left and right channel peak levels in dB.
#[cfg(windows)]
#[tauri::command]
fn get_current_peak(state: tauri::State<AppState>) -> PeakMeterUpdate {
    state.audio_monitor.get_current_peak()
}

#[cfg(not(windows))]
#[tauri::command]
fn get_current_peak() -> Result<(), String> {
    Err("Audio monitoring is only available on Windows".to_string())
}

// =============================================================================
// Application Entry Point
// =============================================================================

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
