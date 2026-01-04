//! Core EQ types and application state.
//!
//! This module defines the fundamental data structures used throughout the EQAPO GUI,
//! including filter types, EQ bands, profiles, and persistent application settings.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[cfg(windows)]
use std::sync::Arc;

#[cfg(windows)]
use crate::audio_monitor::AudioMonitor;

use crate::ab_test::ABSession;

/// Filter types supported by EqualizerAPO.
///
/// These correspond to the standard biquad filter types used in parametric EQ:
/// - `Peaking`: Bell/parametric filter for boosting or cutting a frequency range
/// - `LowShelf`: Boosts or cuts all frequencies below the cutoff
/// - `HighShelf`: Boosts or cuts all frequencies above the cutoff
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterType {
    /// Bell/parametric filter (affects frequencies around the center frequency)
    Peaking,
    /// Low shelf filter (affects frequencies below the cutoff)
    LowShelf,
    /// High shelf filter (affects frequencies above the cutoff)
    HighShelf,
}

impl FilterType {
    /// Converts the filter type to its EqualizerAPO config file abbreviation.
    ///
    /// # Returns
    ///
    /// A static string representing the filter code:
    /// - `Peaking` → `"PK"`
    /// - `LowShelf` → `"LSC"` (Low Shelf with slope in dB/octave)
    /// - `HighShelf` → `"HSC"` (High Shelf with slope in dB/octave)
    pub fn to_eapo_code(&self) -> &'static str {
        match self {
            FilterType::Peaking => "PK",
            FilterType::LowShelf => "LSC",
            FilterType::HighShelf => "HSC",
        }
    }
}

/// A single parametric EQ band with frequency, gain, and Q factor.
///
/// Represents one filter in the EQ chain. Multiple bands can be combined
/// to create a complete EQ profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametricBand {
    /// The type of filter (peaking, low shelf, or high shelf)
    pub filter_type: FilterType,
    /// Center/cutoff frequency in Hz (typically 20-20000)
    pub frequency: f32,
    /// Gain in decibels (positive = boost, negative = cut)
    pub gain: f32,
    /// Q factor controlling bandwidth (higher = narrower, lower = wider)
    pub q_factor: f32,
}

impl ParametricBand {
    /// Formats the band as an EqualizerAPO filter configuration line.
    ///
    /// Generates a string in the format:
    /// `Filter: ON {type} Fc {freq} Hz Gain {gain} dB Q {q}`
    ///
    /// # Returns
    ///
    /// A `String` ready to be written to an EqualizerAPO config file.
    pub fn to_eapo_line(&self) -> String {
        format!(
            "Filter: ON {} Fc {} Hz Gain {:.1} dB Q {:.2}",
            self.filter_type.to_eapo_code(),
            self.frequency as i32,
            self.gain,
            self.q_factor
        )
    }
}

/// An EQ profile containing metadata and a collection of bands.
///
/// Profiles are saved as JSON files in the `Documents/EQAPO GUI/profiles/` directory
/// and can be loaded, saved, and switched between.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqProfile {
    /// Human-readable name of the profile
    pub name: String,
    /// Global preamp gain in dB (applied before all filters)
    #[serde(default)]
    pub preamp: f32,
    /// Collection of parametric EQ bands
    pub bands: Vec<ParametricBand>,
}

/// Persistent application settings saved to `settings.json`.
///
/// This struct serves as the single source of truth for the application state
/// and is automatically persisted when modified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Name of the currently active profile, if any
    #[serde(default)]
    pub current_profile: Option<String>,
    /// Custom config file path (overrides default `live_config.txt`)
    #[serde(default)]
    pub config_path: Option<String>,
    /// Current EQ bands (may differ from saved profile if user modified them)
    #[serde(default = "default_bands")]
    pub bands: Vec<ParametricBand>,
    /// Current preamp value in dB
    #[serde(default)]
    pub preamp: f32,
    /// Whether EQ processing is enabled (false = bypass mode)
    #[serde(default = "default_eq_enabled")]
    pub eq_enabled: bool,
}

/// Returns `true` as the default value for `eq_enabled`.
fn default_eq_enabled() -> bool {
    true
}

/// Creates a default set of EQ bands (single flat peaking filter at 1kHz).
///
/// # Returns
///
/// A `Vec` containing one neutral band at 1000 Hz with 0 dB gain.
pub fn default_bands() -> Vec<ParametricBand> {
    vec![ParametricBand {
        filter_type: FilterType::Peaking,
        frequency: 1000.0,
        gain: 0.0,
        q_factor: 1.41,
    }]
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            current_profile: None,
            config_path: None,
            bands: default_bands(),
            preamp: 0.0,
            eq_enabled: true,
        }
    }
}

/// Application state managed by Tauri.
///
/// Contains all mutable state that persists across command invocations,
/// protected by mutexes for thread-safe access.
pub struct AppState {
    /// Current application settings (thread-safe)
    pub settings: Mutex<AppSettings>,
    /// Active A/B test session, if any (thread-safe)
    pub ab_session: Mutex<Option<ABSession>>,
    /// Audio monitoring interface (Windows only)
    #[cfg(windows)]
    pub audio_monitor: Arc<AudioMonitor>,
}
