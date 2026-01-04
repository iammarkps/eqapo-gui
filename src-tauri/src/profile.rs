//! Profile and settings management for EQAPO GUI.
//!
//! This module handles all file I/O operations for profiles and settings,
//! including loading, saving, and applying EQ configurations to EqualizerAPO.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::tray::update_tray_menu;
use crate::types::{AppSettings, AppState, EqProfile, ParametricBand};

// =============================================================================
// Directory and Path Utilities
// =============================================================================

/// Returns the EQAPO GUI application directory in the user's Documents folder.
///
/// The directory is located at `Documents/EQAPO GUI/` and contains:
/// - `settings.json` - Persistent application settings
/// - `profiles/` - Saved EQ profiles
/// - `ab_results/` - A/B test results
///
/// # Errors
///
/// Returns an error if the Documents folder cannot be determined.
pub fn get_app_dir() -> Result<PathBuf, String> {
    let docs = dirs::document_dir().ok_or("Could not find Documents folder")?;
    Ok(docs.join("EQAPO GUI"))
}

/// Ensures all required application directories exist.
///
/// Creates the following directory structure if it doesn't exist:
/// - `Documents/EQAPO GUI/`
/// - `Documents/EQAPO GUI/profiles/`
///
/// # Returns
///
/// The path to the app directory on success.
///
/// # Errors
///
/// Returns an error if directory creation fails.
pub fn ensure_dirs() -> Result<PathBuf, String> {
    let app_dir = get_app_dir()?;
    let profiles_dir = app_dir.join("profiles");

    fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("Failed to create directories: {}", e))?;

    Ok(app_dir)
}

/// Returns a list of directories where config files are allowed.
///
/// This is a security measure to prevent writing to arbitrary paths.
/// Allowed directories include:
/// - The app directory (`Documents/EQAPO GUI/`)
/// - EqualizerAPO config directory (Windows only)
fn allowed_config_dirs(app_dir: &Path) -> Vec<PathBuf> {
    let mut allowed = vec![app_dir.to_path_buf()];

    #[cfg(windows)]
    {
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            allowed.push(
                PathBuf::from(program_files)
                    .join("EqualizerAPO")
                    .join("config"),
            );
        }
        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            allowed.push(
                PathBuf::from(program_files_x86)
                    .join("EqualizerAPO")
                    .join("config"),
            );
        }
    }

    allowed
}

/// Canonicalizes a target path, handling both existing and non-existing files.
///
/// For existing files, returns the canonical path directly.
/// For non-existing files, canonicalizes the parent directory and appends the filename.
///
/// # Errors
///
/// Returns an error if the path cannot be resolved.
fn canonicalize_target_path(target_path: &Path) -> Result<PathBuf, String> {
    if target_path.exists() {
        return target_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve config path: {}", e));
    }

    let parent = target_path
        .parent()
        .ok_or("Config path has no parent directory")?;
    let parent_canon = parent
        .canonicalize()
        .map_err(|e| format!("Failed to resolve config path directory: {}", e))?;
    let file_name = target_path
        .file_name()
        .ok_or("Config path missing file name")?;

    Ok(parent_canon.join(file_name))
}

/// Validates that a config path is within allowed directories.
///
/// # Arguments
///
/// * `target_path` - The path to validate
/// * `app_dir` - The application directory (used to build allowed list)
///
/// # Returns
///
/// The canonicalized path if valid.
///
/// # Errors
///
/// Returns an error if the path is outside allowed directories.
fn validate_config_path(target_path: &Path, app_dir: &Path) -> Result<PathBuf, String> {
    let canonical_target = canonicalize_target_path(target_path)?;
    let allowed_dirs = allowed_config_dirs(app_dir);
    let canonical_allowed: Vec<PathBuf> = allowed_dirs
        .iter()
        .filter_map(|dir| dir.canonicalize().ok())
        .collect();

    if canonical_allowed
        .iter()
        .any(|dir| canonical_target.starts_with(dir))
    {
        Ok(canonical_target)
    } else {
        Err(format!(
            "Config path {:?} is outside allowed directories",
            target_path
        ))
    }
}

/// Returns the current Windows username from the environment.
#[cfg(windows)]
fn current_windows_user() -> Result<String, String> {
    std::env::var("USERNAME").map_err(|_| "Unable to determine current user".to_string())
}

/// Verifies that a path points to a regular file (not a symlink or directory).
///
/// This is a security measure to prevent symlink-based attacks.
#[cfg(windows)]
fn ensure_regular_file(path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("Failed to inspect config path: {}", e))?;
    if metadata.is_file() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err("Config path is not a regular file".to_string())
    }
}

/// Grants file permissions using Windows `icacls` command.
///
/// # Arguments
///
/// * `path` - The file to modify permissions on
/// * `grant` - The permission string (e.g., `"username:F"` for full access)
///
/// # Errors
///
/// Returns an error if icacls fails or returns a non-zero exit code.
#[cfg(windows)]
fn run_icacls_grant(path: &Path, grant: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("icacls")
        .arg(path)
        .arg("/grant")
        .arg(grant)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run icacls: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("icacls failed: {}", stderr.trim()))
    }
}

// =============================================================================
// Settings Persistence
// =============================================================================

/// Loads application settings from `settings.json`.
///
/// If the file doesn't exist or cannot be parsed, returns default settings.
/// This function never fails - it always returns valid settings.
///
/// # Returns
///
/// The loaded `AppSettings` or defaults if loading fails.
pub fn load_settings() -> AppSettings {
    let app_dir = match get_app_dir() {
        Ok(dir) => dir,
        Err(_) => return AppSettings::default(),
    };

    let settings_path = app_dir.join("settings.json");
    if !settings_path.exists() {
        return AppSettings::default();
    }

    fs::read_to_string(&settings_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Saves application settings to `settings.json`.
///
/// Creates the settings file in `Documents/EQAPO GUI/settings.json`.
/// The file is formatted as pretty-printed JSON for readability.
///
/// # Errors
///
/// Returns an error if:
/// - The app directory cannot be created
/// - JSON serialization fails
/// - File writing fails
pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let app_dir = ensure_dirs()?;
    let settings_path = app_dir.join("settings.json");

    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, json).map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}

// =============================================================================
// Tauri Commands - Profile Management
// =============================================================================

/// Lists all available profile names from the profiles directory.
///
/// Scans `Documents/EQAPO GUI/profiles/` for `.json` files and returns
/// their names (without the `.json` extension).
///
/// # Returns
///
/// A vector of profile names, or an empty vector if no profiles exist.
///
/// # Errors
///
/// Returns an error if the profiles directory cannot be read.
#[tauri::command]
pub fn list_profiles() -> Result<Vec<String>, String> {
    let app_dir = get_app_dir()?;
    let profiles_dir = app_dir.join("profiles");

    if !profiles_dir.exists() {
        return Ok(vec![]);
    }

    let profiles = fs::read_dir(&profiles_dir)
        .map_err(|e| format!("Failed to read profiles directory: {}", e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "json" {
                path.file_stem()?.to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    Ok(profiles)
}

/// Loads a profile by name from the profiles directory.
///
/// # Arguments
///
/// * `name` - The profile name (without `.json` extension)
///
/// # Returns
///
/// The loaded `EqProfile` containing preamp and bands.
///
/// # Errors
///
/// Returns an error if the file doesn't exist or cannot be parsed.
#[tauri::command]
pub fn load_profile(name: String) -> Result<EqProfile, String> {
    let app_dir = get_app_dir()?;
    let profile_path = app_dir.join("profiles").join(format!("{}.json", name));

    let content =
        fs::read_to_string(&profile_path).map_err(|e| format!("Failed to read profile: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse profile: {}", e))
}

/// Saves a profile with the given name, preamp, and bands.
///
/// Creates or overwrites `Documents/EQAPO GUI/profiles/{name}.json`.
///
/// # Arguments
///
/// * `name` - The profile name (used as filename)
/// * `preamp` - Global preamp gain in dB
/// * `bands` - Collection of EQ bands
///
/// # Errors
///
/// Returns an error if the file cannot be written or JSON serialization fails.
#[tauri::command]
pub fn save_profile(name: String, preamp: f32, bands: Vec<ParametricBand>) -> Result<(), String> {
    let app_dir = ensure_dirs()?;
    let profile_path = app_dir.join("profiles").join(format!("{}.json", &name));

    let profile = EqProfile {
        name,
        preamp,
        bands,
    };

    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;

    fs::write(&profile_path, json).map_err(|e| format!("Failed to write profile: {}", e))?;

    Ok(())
}

/// Deletes a profile by name from the profiles directory.
///
/// # Arguments
///
/// * `name` - The profile name to delete
///
/// # Errors
///
/// Returns an error if the file doesn't exist or cannot be deleted.
#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    let app_dir = get_app_dir()?;
    let profile_path = app_dir.join("profiles").join(format!("{}.json", name));

    fs::remove_file(&profile_path).map_err(|e| format!("Failed to delete profile: {}", e))?;

    Ok(())
}

/// Writes EQ bands and preamp to an EqualizerAPO config file.
///
/// Generates an EqualizerAPO-compatible configuration and writes it to
/// the specified path (or `live_config.txt` in the app directory by default).
///
/// On Windows, this function also:
/// - Removes read-only attributes if present
/// - Grants read access to the Windows Audio Service (`NT SERVICE\AudioSrv`)
/// - Retries with elevated permissions if initial write fails
///
/// # Arguments
///
/// * `bands` - Collection of EQ bands to apply
/// * `preamp` - Global preamp gain in dB
/// * `config_path` - Optional custom config file path
/// * `eq_enabled` - Whether EQ is enabled (false = bypass mode)
///
/// # Errors
///
/// Returns an error if:
/// - The config path is outside allowed directories
/// - File writing fails (even after permission fix attempt)
/// - Permission modification fails
#[tauri::command]
pub fn apply_profile(
    bands: Vec<ParametricBand>,
    preamp: f32,
    config_path: Option<String>,
    eq_enabled: Option<bool>,
) -> Result<(), String> {
    let app_dir = ensure_dirs()?;
    let target_path = config_path
        .map(PathBuf::from)
        .unwrap_or_else(|| app_dir.join("live_config.txt"));

    let target_path = validate_config_path(&target_path, &app_dir)?;

    // Build EqualizerAPO config content
    let enabled = eq_enabled.unwrap_or(true);
    let content = if enabled {
        let mut lines = vec![
            String::from("; EQAPO GUI Live Configuration"),
            String::from("; Auto-generated - do not edit manually"),
            String::from(""),
            format!("Preamp: {:.1} dB", preamp),
            String::from(""),
        ];

        for band in &bands {
            lines.push(band.to_eapo_line());
        }

        lines.join("\r\n")
    } else {
        // EQ disabled - write empty config (bypassed)
        [
            "; EQAPO GUI Live Configuration",
            "; EQ DISABLED - Bypass mode",
            "",
            "; No filters applied",
        ]
        .join("\r\n")
    };

    // Try to remove readonly attribute if file exists (Windows-specific behavior)
    #[allow(clippy::permissions_set_readonly_false)] // This is Windows-only, Unix warning N/A
    if target_path.exists() {
        if let Ok(metadata) = fs::metadata(&target_path) {
            let mut perms = metadata.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(&target_path, perms);
            }
        }
    }

    // Attempt to write
    if let Err(e) = fs::write(&target_path, &content) {
        // If write fails, try to force permissions via icacls BEFORE failing
        #[cfg(target_os = "windows")]
        {
            if target_path.exists() {
                ensure_regular_file(&target_path)?;
                let user = current_windows_user()?;
                run_icacls_grant(&target_path, &format!("{}:F", user))?;
            }
        }

        // Retry write once
        fs::write(&target_path, &content).map_err(|retry_err| {
            format!(
                "Failed to write to {:?}: {} (Retry: {})",
                target_path, e, retry_err
            )
        })?;
    }

    // Fix permissions for EqualizerAPO (Windows Audio Service needs access)
    #[cfg(target_os = "windows")]
    {
        ensure_regular_file(&target_path)?;
        run_icacls_grant(&target_path, "NT SERVICE\\AudioSrv:R")?;
    }

    Ok(())
}

// =============================================================================
// Tauri Commands - Settings Management
// =============================================================================

/// Returns the name of the currently active profile.
///
/// # Returns
///
/// `Some(name)` if a profile is active, `None` otherwise.
#[tauri::command]
pub fn get_current_profile(state: tauri::State<AppState>) -> Option<String> {
    state.settings.lock().current_profile.clone()
}

/// Sets the current profile and persists the change.
///
/// Updates the application state and settings file, then refreshes
/// the system tray menu to reflect the new selection.
///
/// # Arguments
///
/// * `name` - The profile name to set, or `None` to clear
/// * `state` - Tauri managed state
/// * `app` - Tauri app handle for tray updates
///
/// # Errors
///
/// Returns an error if settings cannot be saved.
#[tauri::command]
pub fn set_current_profile(
    name: Option<String>,
    state: tauri::State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock();
        settings.current_profile = name;
        save_settings(&settings)?;
    }
    let _ = update_tray_menu(&app);
    Ok(())
}

/// Returns all current application settings.
///
/// # Returns
///
/// A clone of the current `AppSettings`.
#[tauri::command]
pub fn get_settings(state: tauri::State<AppState>) -> AppSettings {
    state.settings.lock().clone()
}

/// Updates application settings from the frontend UI.
///
/// Called whenever the UI state changes to keep settings in sync.
/// Persists changes to disk and updates the tray menu.
///
/// # Arguments
///
/// * `bands` - Current EQ bands
/// * `preamp` - Current preamp value
/// * `current_profile` - Currently active profile name
/// * `config_path` - Custom config file path
/// * `eq_enabled` - Whether EQ is enabled
/// * `state` - Tauri managed state
/// * `app` - Tauri app handle for tray updates
///
/// # Errors
///
/// Returns an error if settings cannot be saved.
#[tauri::command]
pub fn update_settings(
    bands: Vec<ParametricBand>,
    preamp: f32,
    current_profile: Option<String>,
    config_path: Option<String>,
    eq_enabled: Option<bool>,
    state: tauri::State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock();
        settings.bands = bands;
        settings.preamp = preamp;
        settings.current_profile = current_profile;
        settings.config_path = config_path;
        if let Some(enabled) = eq_enabled {
            settings.eq_enabled = enabled;
        }
        save_settings(&settings)?;
    }
    let _ = update_tray_menu(&app);
    Ok(())
}
