//! Core EQ types and application state.
//!
//! This module defines the fundamental data structures used throughout the EQAPO GUI
//! application. It serves as the single source of truth for type definitions that are
//! shared between the Tauri backend and the frontend via IPC.
//!
//! # Overview
//!
//! The type hierarchy is organized as follows:
//!
//! - [`FilterType`] - Enumeration of supported biquad filter types
//! - [`ParametricBand`] - A single EQ band with frequency, gain, and Q parameters
//! - [`EqProfile`] - A named collection of bands representing a complete EQ curve
//! - [`AppSettings`] - Persistent application configuration
//! - [`AppState`] - Runtime state managed by Tauri
//!
//! # Serialization
//!
//! All public types implement [`Serialize`] and [`Deserialize`] for JSON serialization.
//! Filter types use lowercase naming (e.g., `"peaking"`, `"lowshelf"`) for frontend
//! compatibility.
//!
//! # Example
//!
//! ```ignore
//! use eqapo_gui_lib::{FilterType, ParametricBand, EqProfile};
//!
//! let band = ParametricBand {
//!     filter_type: FilterType::Peaking,
//!     frequency: 1000.0,
//!     gain: 3.0,
//!     q_factor: 1.41,
//! };
//!
//! let profile = EqProfile {
//!     name: "My Profile".to_string(),
//!     preamp: -3.0,
//!     bands: vec![band],
//! };
//! ```

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::sync::Arc;

#[cfg(windows)]
use crate::audio_monitor::AudioMonitor;

use crate::ab_test::ABSession;

/// Filter types supported by EqualizerAPO.
///
/// These correspond to the standard [biquad filter] types used in parametric equalizers.
/// Each filter type shapes the frequency response differently, allowing precise control
/// over the audio spectrum.
///
/// [biquad filter]: https://en.wikipedia.org/wiki/Digital_biquad_filter
///
/// # EqualizerAPO Mapping
///
/// | Variant     | EAPO Code | Description                          |
/// |-------------|-----------|--------------------------------------|
/// | `Peaking`   | `PK`      | Bell curve centered on frequency     |
/// | `LowShelf`  | `LSC`     | Shelf affecting low frequencies      |
/// | `HighShelf` | `HSC`     | Shelf affecting high frequencies     |
///
/// # Serialization
///
/// Serializes to lowercase strings for JSON compatibility:
/// - `Peaking` → `"peaking"`
/// - `LowShelf` → `"lowshelf"`
/// - `HighShelf` → `"highshelf"`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterType {
    /// Bell/parametric filter centered on the frequency.
    ///
    /// Creates a symmetrical boost or cut around the center frequency.
    /// The Q factor controls the bandwidth - higher Q means narrower bandwidth.
    /// This is the most common filter type for surgical EQ adjustments.
    Peaking,

    /// Low shelf filter affecting frequencies below the cutoff.
    ///
    /// Applies a constant gain to all frequencies below the cutoff frequency,
    /// with a smooth transition around the cutoff point. The Q factor controls
    /// the steepness of the transition slope.
    LowShelf,

    /// High shelf filter affecting frequencies above the cutoff.
    ///
    /// Applies a constant gain to all frequencies above the cutoff frequency,
    /// with a smooth transition around the cutoff point. The Q factor controls
    /// the steepness of the transition slope.
    HighShelf,
}

impl FilterType {
    /// Converts the filter type to its EqualizerAPO configuration file abbreviation.
    ///
    /// These codes are used when generating EqualizerAPO filter lines in the format:
    /// `Filter: ON {code} Fc {freq} Hz Gain {gain} dB Q {q}`
    ///
    /// # Returns
    ///
    /// A static string slice containing the two or three character filter code:
    ///
    /// | Filter Type | Code  | Meaning                        |
    /// |-------------|-------|--------------------------------|
    /// | `Peaking`   | `PK`  | Peaking (parametric) filter    |
    /// | `LowShelf`  | `LSC` | Low Shelf with Q (slope) control |
    /// | `HighShelf` | `HSC` | High Shelf with Q (slope) control |
    ///
    /// # Example
    ///
    /// ```
    /// use eqapo_gui_lib::FilterType;
    ///
    /// assert_eq!(FilterType::Peaking.to_eapo_code(), "PK");
    /// assert_eq!(FilterType::LowShelf.to_eapo_code(), "LSC");
    /// ```
    #[inline]
    #[must_use]
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
/// to create a complete EQ profile. Each band independently modifies the
/// frequency response at its specified frequency.
///
/// # Parameters
///
/// - **Frequency**: The center (for peaking) or cutoff (for shelves) frequency in Hz.
///   Valid range is typically 20 Hz to 20,000 Hz (human hearing range).
///
/// - **Gain**: The amount of boost or cut in decibels (dB).
///   - Positive values boost (increase volume at that frequency)
///   - Negative values cut (decrease volume at that frequency)
///   - Typical range: -12 dB to +12 dB
///
/// - **Q Factor**: Controls the bandwidth (width) of the filter.
///   - Higher Q = narrower bandwidth (more surgical)
///   - Lower Q = wider bandwidth (more gentle)
///   - Typical range: 0.1 to 10.0
///   - Common default: 1.41 (≈ √2, one octave bandwidth for peaking)
///
/// # Example
///
/// ```
/// use eqapo_gui_lib::{FilterType, ParametricBand};
///
/// // Create a bass boost at 100 Hz
/// let bass_boost = ParametricBand {
///     filter_type: FilterType::LowShelf,
///     frequency: 100.0,
///     gain: 4.0,
///     q_factor: 0.71,
/// };
///
/// // Create a narrow cut to remove a resonance at 3.5 kHz
/// let notch = ParametricBand {
///     filter_type: FilterType::Peaking,
///     frequency: 3500.0,
///     gain: -6.0,
///     q_factor: 8.0,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametricBand {
    /// The type of filter determining how the frequency response is shaped.
    ///
    /// See [`FilterType`] for available options.
    pub filter_type: FilterType,

    /// Center frequency (peaking) or cutoff frequency (shelves) in Hertz.
    ///
    /// Valid range: 20.0 to 20000.0 Hz (human hearing range).
    /// Values outside this range may be accepted but won't be audible.
    pub frequency: f32,

    /// Gain adjustment in decibels (dB).
    ///
    /// - Positive values: boost (increase level)
    /// - Negative values: cut (decrease level)
    /// - Zero: no change (filter has no effect)
    ///
    /// Typical range: -24.0 to +24.0 dB, though ±12 dB is more common.
    pub gain: f32,

    /// Q factor (quality factor) controlling the filter bandwidth.
    ///
    /// For peaking filters:
    /// - Q ≈ 0.7: ~2 octave bandwidth (very wide)
    /// - Q ≈ 1.4: ~1 octave bandwidth (moderate)
    /// - Q ≈ 4.0: ~1/3 octave bandwidth (narrow)
    /// - Q ≈ 10.0: very narrow (surgical)
    ///
    /// For shelf filters, Q controls the transition slope steepness.
    pub q_factor: f32,
}

impl ParametricBand {
    /// Formats the band as an EqualizerAPO filter configuration line.
    ///
    /// Generates a configuration string that EqualizerAPO can parse and apply.
    /// The format follows EqualizerAPO's filter syntax specification.
    ///
    /// # Output Format
    ///
    /// ```text
    /// Filter: ON {type} Fc {freq} Hz Gain {gain} dB Q {q}
    /// ```
    ///
    /// Where:
    /// - `{type}` is the filter code (`PK`, `LSC`, or `HSC`)
    /// - `{freq}` is the frequency as an integer (truncated, not rounded)
    /// - `{gain}` is the gain with one decimal place
    /// - `{q}` is the Q factor with two decimal places
    ///
    /// # Example
    ///
    /// ```
    /// use eqapo_gui_lib::{FilterType, ParametricBand};
    ///
    /// let band = ParametricBand {
    ///     filter_type: FilterType::Peaking,
    ///     frequency: 1000.0,
    ///     gain: 3.5,
    ///     q_factor: 1.41,
    /// };
    ///
    /// assert_eq!(
    ///     band.to_eapo_line(),
    ///     "Filter: ON PK Fc 1000 Hz Gain 3.5 dB Q 1.41"
    /// );
    /// ```
    ///
    /// # Note
    ///
    /// The frequency is truncated to an integer because EqualizerAPO
    /// does not support fractional Hz values in its configuration files.
    #[must_use]
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

/// A complete EQ profile containing a name, preamp setting, and collection of bands.
///
/// Profiles represent saved EQ configurations that users can create, load, and switch
/// between. Each profile is stored as a JSON file in the application's profiles directory.
///
/// # Storage Location
///
/// Profiles are saved to: `Documents/EQAPO GUI/profiles/{name}.json`
///
/// # JSON Format
///
/// ```json
/// {
///   "name": "My Profile",
///   "preamp": -3.0,
///   "bands": [
///     {
///       "filter_type": "peaking",
///       "frequency": 1000.0,
///       "gain": 3.0,
///       "q_factor": 1.41
///     }
///   ]
/// }
/// ```
///
/// # Preamp Usage
///
/// The preamp value is applied globally before any filters. It's commonly used to:
/// - Prevent clipping when boosting frequencies (set negative preamp)
/// - Increase overall volume when cutting frequencies
/// - Match loudness between different profiles
///
/// # Example
///
/// ```
/// use eqapo_gui_lib::{EqProfile, FilterType, ParametricBand};
///
/// let profile = EqProfile {
///     name: "Vocal Presence".to_string(),
///     preamp: -2.0,  // Reduce by 2dB to prevent clipping
///     bands: vec![
///         ParametricBand {
///             filter_type: FilterType::Peaking,
///             frequency: 3000.0,
///             gain: 4.0,
///             q_factor: 1.0,
///         },
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqProfile {
    /// Human-readable name identifying the profile.
    ///
    /// This name is used as the filename (with `.json` extension) when saving,
    /// and displayed in the UI for profile selection.
    pub name: String,

    /// Global preamp gain in decibels applied before all filters.
    ///
    /// - Negative values reduce overall volume (prevents clipping with boosts)
    /// - Positive values increase overall volume
    /// - Defaults to 0.0 if not specified in JSON
    #[serde(default)]
    pub preamp: f32,

    /// Collection of EQ bands that define the frequency response curve.
    ///
    /// Bands are applied in order, though for linear-phase EQ the order
    /// typically doesn't affect the final result.
    pub bands: Vec<ParametricBand>,
}

/// Persistent application settings saved to `settings.json`.
///
/// This struct serves as the single source of truth for the application's persistent
/// state. It is automatically loaded on startup and saved whenever settings change,
/// ensuring the EQ configuration persists across application restarts.
///
/// # Storage Location
///
/// Settings file: `Documents/EQAPO GUI/settings.json`
///
/// # Relationship to Profiles
///
/// The settings store the *current* EQ configuration, which may differ from any
/// saved profile. When a user:
/// - Loads a profile: bands and preamp are copied to settings
/// - Modifies the EQ: settings are updated, but the profile file is unchanged
/// - Saves a profile: current bands/preamp are written to the profile file
///
/// # Default Values
///
/// When deserializing, missing fields receive sensible defaults:
/// - `current_profile`: `None`
/// - `config_path`: `None` (uses default `live_config.txt`)
/// - `bands`: Single flat band at 1 kHz
/// - `preamp`: `0.0` dB
/// - `eq_enabled`: `true`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Name of the currently active profile, if any.
    ///
    /// This is displayed in the UI and system tray, and is used to show
    /// a checkmark next to the active profile in menus.
    ///
    /// Set to `None` when:
    /// - No profile has been loaded
    /// - The user has modified settings after loading (optional behavior)
    #[serde(default)]
    pub current_profile: Option<String>,

    /// Custom path to the EqualizerAPO configuration file.
    ///
    /// If `None`, defaults to `Documents/EQAPO GUI/live_config.txt`.
    /// Set this to write directly to EqualizerAPO's config directory, e.g.,
    /// `C:\Program Files\EqualizerAPO\config\live_config.txt`
    ///
    /// # Security
    ///
    /// Paths are validated to ensure they're within allowed directories
    /// (app directory or EqualizerAPO config folder) to prevent path
    /// traversal attacks.
    #[serde(default)]
    pub config_path: Option<String>,

    /// Current EQ bands representing the active frequency response curve.
    ///
    /// These bands are written to the EqualizerAPO config file whenever
    /// the EQ is applied. May differ from any saved profile if the user
    /// has made modifications.
    #[serde(default = "default_bands")]
    pub bands: Vec<ParametricBand>,

    /// Current preamp value in decibels.
    ///
    /// Applied globally before all filters. Used to prevent clipping
    /// when boosting frequencies.
    #[serde(default)]
    pub preamp: f32,

    /// Whether EQ processing is currently enabled.
    ///
    /// When `false`, the EQ is bypassed and an empty configuration is
    /// written to EqualizerAPO (no filters applied). This allows quick
    /// A/B comparison between processed and unprocessed audio.
    #[serde(default = "default_eq_enabled")]
    pub eq_enabled: bool,
}

/// Default value provider for `eq_enabled` field during deserialization.
///
/// Returns `true` because EQ should be enabled by default when a user
/// first launches the application or when the field is missing from
/// saved settings.
#[inline]
fn default_eq_enabled() -> bool {
    true
}

/// Creates a default set of EQ bands for new configurations.
///
/// The default configuration provides a single neutral (0 dB) peaking filter
/// at 1 kHz, which serves as a starting point for users to begin creating
/// their EQ curve.
///
/// # Returns
///
/// A [`Vec`] containing one [`ParametricBand`] with:
/// - Type: Peaking
/// - Frequency: 1000 Hz
/// - Gain: 0.0 dB (neutral)
/// - Q Factor: 1.41 (~1 octave bandwidth)
///
/// # Usage
///
/// This function is used as the serde default for [`AppSettings::bands`]
/// and can also be called directly when initializing new configurations.
#[must_use]
pub fn default_bands() -> Vec<ParametricBand> {
    vec![ParametricBand {
        filter_type: FilterType::Peaking,
        frequency: 1000.0,
        gain: 0.0,
        q_factor: 1.41,
    }]
}

impl Default for AppSettings {
    /// Creates default application settings for first-time launch.
    ///
    /// # Default Values
    ///
    /// | Field           | Value                      |
    /// |-----------------|----------------------------|
    /// | current_profile | `None`                     |
    /// | config_path     | `None`                     |
    /// | bands           | Single band at 1 kHz       |
    /// | preamp          | 0.0 dB                     |
    /// | eq_enabled      | `true`                     |
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

/// Runtime application state managed by Tauri's state management system.
///
/// This struct holds all mutable state that needs to persist across Tauri command
/// invocations. Each field is protected by a [`Mutex`] for thread-safe access from
/// multiple concurrent command handlers.
///
/// # Thread Safety
///
/// Uses [`parking_lot::Mutex`] which:
/// - Does not poison on panic (unlike `std::sync::Mutex`)
/// - Provides better performance than the standard library mutex
/// - Returns the guard directly from `lock()` without `Result`
///
/// # Usage
///
/// This struct is registered with Tauri using `.manage()` and accessed in
/// command handlers via `tauri::State<AppState>`:
///
/// ```ignore
/// #[tauri::command]
/// fn my_command(state: tauri::State<AppState>) {
///     let settings = state.settings.lock();
///     // Use settings...
/// }
/// ```
///
/// # Platform-Specific Fields
///
/// The `audio_monitor` field is only available on Windows, where WASAPI
/// provides audio loopback capture for peak metering.
pub struct AppState {
    /// Current application settings, persisted to disk.
    ///
    /// Lock this mutex briefly when reading or updating settings.
    /// The settings are automatically saved to disk after modifications.
    pub settings: Mutex<AppSettings>,

    /// Active A/B test session, if one is in progress.
    ///
    /// Contains `Some(session)` during blind listening tests,
    /// `None` when no test is active. The session tracks trial
    /// progress, randomization, and user responses.
    pub ab_session: Mutex<Option<ABSession>>,

    /// Audio monitoring interface for peak metering (Windows only).
    ///
    /// Provides real-time audio level monitoring via WASAPI loopback capture.
    /// Wrapped in [`Arc`] for safe sharing across threads (the monitoring
    /// runs on a background thread).
    #[cfg(windows)]
    pub audio_monitor: Arc<AudioMonitor>,
}
