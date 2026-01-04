//! A/B test Tauri commands.
//!
//! This module provides the Tauri command handlers for A/B and blind listening tests,
//! allowing users to compare EQ presets and measure their ability to distinguish them.

use std::fs;

use crate::ab_test::{
    export_results_csv, export_results_json, ABSession, ABSessionResults, ABStateForUI, ABTestMode,
    ActiveOption,
};
use crate::profile::{apply_profile, get_app_dir, load_profile};
use crate::types::AppState;

/// Starts a new A/B test session.
///
/// Initializes a new session with the specified test mode and presets.
/// The session manages trial progression, randomization (for blind tests),
/// and result tracking.
///
/// # Arguments
///
/// * `mode` - The test mode (`AB`, `BlindAB`, or `ABX`)
/// * `preset_a` - Name of the first preset to compare
/// * `preset_b` - Name of the second preset to compare
/// * `total_trials` - Number of trials to run (only used in blind/ABX modes)
/// * `trim_db` - Optional loudness trim applied to preset B
/// * `state` - Tauri managed state
///
/// # Returns
///
/// The initial UI state for the test session.
///
/// # Errors
///
/// Returns an error if the session cannot be created (e.g., invalid presets).
#[tauri::command]
pub fn start_ab_session(
    mode: ABTestMode,
    preset_a: String,
    preset_b: String,
    total_trials: usize,
    trim_db: Option<f32>,
    state: tauri::State<AppState>,
) -> Result<ABStateForUI, String> {
    let session = ABSession::new(mode, preset_a, preset_b, total_trials, trim_db)?;
    let ui_state = session.get_ui_state();

    // Store the session
    let mut ab_guard = state.ab_session.lock();
    *ab_guard = Some(session);

    Ok(ui_state)
}

/// Applies an A/B test option by switching to the specified preset.
///
/// Loads the preset associated with the given option and applies it
/// to the EqualizerAPO configuration. In blind modes, options "1" and "2"
/// are mapped to randomized presets.
///
/// # Arguments
///
/// * `option` - The option to apply:
///   - `"A"` - Apply preset A (with trim for A)
///   - `"B"` - Apply preset B (with trim for B)
///   - `"X"` - Apply the mystery preset (ABX mode)
///   - `"1"` - Apply blind option 1 (randomized)
///   - `"2"` - Apply blind option 2 (randomized)
/// * `state` - Tauri managed state
///
/// # Errors
///
/// Returns an error if:
/// - No active A/B session exists
/// - The option is invalid
/// - The preset cannot be loaded or applied
#[tauri::command]
pub fn apply_ab_option(option: String, state: tauri::State<AppState>) -> Result<(), String> {
    // First, get config_path from settings (short lock scope)
    let config_path = {
        let settings = state.settings.lock();
        settings.config_path.clone()
    };

    // Then work with the A/B session
    let (preset_name, trim) = {
        let mut ab_guard = state.ab_session.lock();
        let session = ab_guard.as_mut().ok_or("No active A/B session")?;

        let (preset_name, trim) = match option.as_str() {
            "A" => {
                session.active_option = Some(ActiveOption::A);
                session.get_preset_for_option(ActiveOption::A)
            }
            "B" => {
                session.active_option = Some(ActiveOption::B);
                session.get_preset_for_option(ActiveOption::B)
            }
            "X" => {
                session.active_option = Some(ActiveOption::X);
                session.get_preset_for_option(ActiveOption::X)
            }
            "1" => {
                session.active_option = Some(ActiveOption::A);
                session.get_preset_for_blind_option(1)
            }
            "2" => {
                session.active_option = Some(ActiveOption::B);
                session.get_preset_for_blind_option(2)
            }
            _ => return Err(format!("Invalid option: {}", option)),
        };

        (preset_name.to_string(), trim)
    };

    // Now load and apply the profile (no locks held)
    let profile = load_profile(preset_name)?;
    let adjusted_preamp = profile.preamp + trim;
    apply_profile(profile.bands, adjusted_preamp, config_path, Some(true))?;

    Ok(())
}

/// Records the user's answer for the current trial.
///
/// In blind and ABX modes, records whether the user correctly identified
/// the preset. Advances to the next trial if applicable.
///
/// # Arguments
///
/// * `answer` - The user's answer (e.g., `"A"`, `"B"`, `"1"`, `"2"`)
/// * `state` - Tauri managed state
///
/// # Returns
///
/// The updated UI state after recording the answer.
///
/// # Errors
///
/// Returns an error if no active session exists or the answer is invalid.
#[tauri::command]
pub fn record_ab_answer(
    answer: String,
    state: tauri::State<AppState>,
) -> Result<ABStateForUI, String> {
    let mut ab_guard = state.ab_session.lock();
    let session = ab_guard.as_mut().ok_or("No active A/B session")?;

    session.record_answer(answer)?;
    Ok(session.get_ui_state())
}

/// Returns the current A/B session state for the UI.
///
/// # Returns
///
/// `Some(state)` if a session is active, `None` otherwise.
#[tauri::command]
pub fn get_ab_state(state: tauri::State<AppState>) -> Option<ABStateForUI> {
    let ab_guard = state.ab_session.lock();
    ab_guard.as_ref().map(|s| s.get_ui_state())
}

/// Finishes the A/B session and exports results to files.
///
/// Ends the current session, calculates final statistics (including
/// binomial p-value for statistical significance), and exports results
/// to both JSON and CSV formats in `Documents/EQAPO GUI/ab_results/`.
///
/// # Arguments
///
/// * `state` - Tauri managed state
///
/// # Returns
///
/// The complete session results including trial data and statistics.
///
/// # Errors
///
/// Returns an error if:
/// - No active session exists
/// - Results directory cannot be created
/// - File export fails
#[tauri::command]
pub fn finish_ab_session(state: tauri::State<AppState>) -> Result<ABSessionResults, String> {
    let mut ab_guard = state.ab_session.lock();
    let session = ab_guard.take().ok_or("No active A/B session")?;

    let results = session.get_results();

    // Export to files
    let app_dir = get_app_dir()?;
    let results_dir = app_dir.join("ab_results");
    fs::create_dir_all(&results_dir).map_err(|e| format!("Failed to create results dir: {}", e))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // JSON export
    let json_path = results_dir.join(format!("session_{}.json", timestamp));
    let json_content = export_results_json(&results)?;
    fs::write(&json_path, json_content).map_err(|e| format!("Failed to write JSON: {}", e))?;

    // CSV export
    let csv_path = results_dir.join(format!("session_{}.csv", timestamp));
    let csv_content = export_results_csv(&results);
    fs::write(&csv_path, csv_content).map_err(|e| format!("Failed to write CSV: {}", e))?;

    Ok(results)
}

/// Updates the loudness trim value during an active session.
///
/// Allows real-time adjustment of the volume difference between presets
/// to achieve loudness-matched comparisons.
///
/// # Arguments
///
/// * `trim_db` - The new trim value in dB (applied to preset B)
/// * `state` - Tauri managed state
///
/// # Errors
///
/// Returns an error if no active session exists.
#[tauri::command]
pub fn update_ab_trim(trim_db: f32, state: tauri::State<AppState>) -> Result<(), String> {
    let mut ab_guard = state.ab_session.lock();
    let session = ab_guard.as_mut().ok_or("No active A/B session")?;

    session.trim_db = trim_db;
    Ok(())
}
