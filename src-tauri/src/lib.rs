use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

mod ab_test;
use ab_test::{
    export_results_csv, export_results_json, ABSession, ABSessionResults, ABStateForUI, ABTestMode,
    ActiveOption,
};

#[cfg(windows)]
mod audio_monitor;
#[cfg(windows)]
use audio_monitor::{AudioMonitor, AudioOutputInfo, PeakMeterUpdate};
#[cfg(windows)]
use std::sync::Arc;

/// Filter types supported by EqualizerAPO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
}

impl FilterType {
    /// Convert to EqualizerAPO syntax abbreviation
    fn to_eapo_code(&self) -> &'static str {
        match self {
            FilterType::Peaking => "PK",
            FilterType::LowShelf => "LSC",
            FilterType::HighShelf => "HSC",
        }
    }
}

/// A single parametric EQ band
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametricBand {
    pub filter_type: FilterType,
    pub frequency: f32,
    pub gain: f32,
    pub q_factor: f32,
}

impl ParametricBand {
    /// Format band as EqualizerAPO filter line
    fn to_eapo_line(&self) -> String {
        format!(
            "Filter: ON {} Fc {} Hz Gain {:.1} dB Q {:.2}",
            self.filter_type.to_eapo_code(),
            self.frequency as i32,
            self.gain,
            self.q_factor
        )
    }
}

/// EQ Profile containing metadata and bands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqProfile {
    pub name: String,
    #[serde(default)]
    pub preamp: f32,
    pub bands: Vec<ParametricBand>,
}

/// Application settings for persistence (single source of truth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub current_profile: Option<String>,
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default = "default_bands")]
    pub bands: Vec<ParametricBand>,
    #[serde(default)]
    pub preamp: f32,
    #[serde(default = "default_eq_enabled")]
    pub eq_enabled: bool,
}

fn default_eq_enabled() -> bool {
    true
}

fn default_bands() -> Vec<ParametricBand> {
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

/// Managed state for tracking current settings
pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub ab_session: Mutex<Option<ABSession>>,
    #[cfg(windows)]
    pub audio_monitor: Arc<AudioMonitor>,
}

/// Get the EQAPO GUI directory in Documents
fn get_app_dir() -> Result<PathBuf, String> {
    let docs = dirs::document_dir().ok_or("Could not find Documents folder")?;
    Ok(docs.join("EQAPO GUI"))
}

/// Ensure all required directories exist
fn ensure_dirs() -> Result<PathBuf, String> {
    let app_dir = get_app_dir()?;
    let profiles_dir = app_dir.join("profiles");

    fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("Failed to create directories: {}", e))?;

    Ok(app_dir)
}

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

#[cfg(windows)]
fn current_windows_user() -> Result<String, String> {
    std::env::var("USERNAME").map_err(|_| "Unable to determine current user".to_string())
}

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

/// Load settings from settings.json
fn load_settings() -> AppSettings {
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

/// Save settings to settings.json
fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let app_dir = ensure_dirs()?;
    let settings_path = app_dir.join("settings.json");

    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, json).map_err(|e| format!("Failed to write settings: {}", e))?;

    Ok(())
}

/// List all available profile names
#[tauri::command]
fn list_profiles() -> Result<Vec<String>, String> {
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

/// Load a profile by name
#[tauri::command]
fn load_profile(name: String) -> Result<EqProfile, String> {
    let app_dir = get_app_dir()?;
    let profile_path = app_dir.join("profiles").join(format!("{}.json", name));

    let content =
        fs::read_to_string(&profile_path).map_err(|e| format!("Failed to read profile: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse profile: {}", e))
}

/// Save a profile with the given name, preamp, and bands
#[tauri::command]
fn save_profile(name: String, preamp: f32, bands: Vec<ParametricBand>) -> Result<(), String> {
    let app_dir = ensure_dirs()?;
    let profile_path = app_dir.join("profiles").join(format!("{}.json", name));

    let profile = EqProfile {
        name: name.clone(),
        preamp,
        bands,
    };

    let json = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;

    fs::write(&profile_path, json).map_err(|e| format!("Failed to write profile: {}", e))?;

    Ok(())
}

/// Apply bands to live_config.txt for EqualizerAPO
#[tauri::command]
fn apply_profile(
    bands: Vec<ParametricBand>,
    preamp: f32,
    config_path: Option<String>,
    eq_enabled: Option<bool>,
) -> Result<(), String> {
    let app_dir = ensure_dirs()?;
    let target_path = if let Some(path) = config_path {
        PathBuf::from(path)
    } else {
        app_dir.join("live_config.txt")
    };

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
        vec![
            String::from("; EQAPO GUI Live Configuration"),
            String::from("; EQ DISABLED - Bypass mode"),
            String::from(""),
            String::from("; No filters applied"),
        ]
        .join("\r\n")
    };

    // Try to remove readonly attribute if file exists
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

/// Delete a profile by name
#[tauri::command]
fn delete_profile(name: String) -> Result<(), String> {
    let app_dir = get_app_dir()?;
    let profile_path = app_dir.join("profiles").join(format!("{}.json", name));

    fs::remove_file(&profile_path).map_err(|e| format!("Failed to delete profile: {}", e))?;

    Ok(())
}

/// Get the current active profile name
#[tauri::command]
fn get_current_profile(state: tauri::State<AppState>) -> Option<String> {
    state.settings.lock().ok()?.current_profile.clone()
}

/// Set the current profile and update settings
#[tauri::command]
fn set_current_profile(
    name: Option<String>,
    state: tauri::State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Update state
    if let Ok(mut settings) = state.settings.lock() {
        settings.current_profile = name.clone();
        save_settings(&settings)?;
    }

    // Update tray menu
    let _ = update_tray_menu(&app);

    Ok(())
}

/// Get all settings
#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> Result<AppSettings, String> {
    state
        .settings
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "Failed to lock settings".to_string())
}

/// Update settings (called when UI state changes)
#[tauri::command]
fn update_settings(
    bands: Vec<ParametricBand>,
    preamp: f32,
    current_profile: Option<String>,
    config_path: Option<String>,
    eq_enabled: Option<bool>,
    state: tauri::State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if let Ok(mut settings) = state.settings.lock() {
        settings.bands = bands;
        settings.preamp = preamp;
        settings.current_profile = current_profile;
        settings.config_path = config_path;
        if let Some(enabled) = eq_enabled {
            settings.eq_enabled = enabled;
        }
        save_settings(&settings)?;
    }

    // Update tray menu in case profile changed
    let _ = update_tray_menu(&app);

    Ok(())
}

// ============================================================================
// A/B Test Commands
// ============================================================================

/// Start a new A/B test session
#[tauri::command]
fn start_ab_session(
    mode: ABTestMode,
    preset_a: String,
    preset_b: String,
    total_trials: usize,
    trim_db: Option<f32>,
    state: tauri::State<AppState>,
) -> Result<ABStateForUI, String> {
    let session = ABSession::new(mode, preset_a, preset_b, total_trials, trim_db)?;
    let ui_state = session.get_ui_state();

    if let Ok(mut ab) = state.ab_session.lock() {
        *ab = Some(session);
    }

    Ok(ui_state)
}

/// Apply an A/B option (switch presets)
#[tauri::command]
fn apply_ab_option(
    option: String, // "A", "B", "X", "1", "2"
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut ab_guard = state
        .ab_session
        .lock()
        .map_err(|_| "Failed to lock session")?;
    let session = ab_guard.as_mut().ok_or("No active A/B session")?;

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Failed to lock settings")?;
    let config_path = settings.config_path.clone();
    drop(settings);

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
            session.active_option = Some(ActiveOption::A); // Will be hidden in UI
            session.get_preset_for_blind_option(1)
        }
        "2" => {
            session.active_option = Some(ActiveOption::B); // Will be hidden in UI
            session.get_preset_for_blind_option(2)
        }
        _ => return Err(format!("Invalid option: {}", option)),
    };

    // Load and apply the preset with trim
    let profile = load_profile(preset_name.to_string())?;
    let adjusted_preamp = profile.preamp + trim;

    drop(ab_guard); // Release lock before apply_profile

    apply_profile(profile.bands, adjusted_preamp, config_path, Some(true))?;

    Ok(())
}

/// Record user's answer for current trial
#[tauri::command]
fn record_ab_answer(answer: String, state: tauri::State<AppState>) -> Result<ABStateForUI, String> {
    let mut ab_guard = state
        .ab_session
        .lock()
        .map_err(|_| "Failed to lock session")?;
    let session = ab_guard.as_mut().ok_or("No active A/B session")?;

    session.record_answer(answer)?;
    Ok(session.get_ui_state())
}

/// Get current A/B session state
#[tauri::command]
fn get_ab_state(state: tauri::State<AppState>) -> Result<Option<ABStateForUI>, String> {
    let ab_guard = state
        .ab_session
        .lock()
        .map_err(|_| "Failed to lock session")?;

    Ok(ab_guard.as_ref().map(|s| s.get_ui_state()))
}

/// Finish session and export results
#[tauri::command]
fn finish_ab_session(state: tauri::State<AppState>) -> Result<ABSessionResults, String> {
    let mut ab_guard = state
        .ab_session
        .lock()
        .map_err(|_| "Failed to lock session")?;
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

/// Update trim during session
#[tauri::command]
fn update_ab_trim(trim_db: f32, state: tauri::State<AppState>) -> Result<(), String> {
    let mut ab_guard = state
        .ab_session
        .lock()
        .map_err(|_| "Failed to lock session")?;
    let session = ab_guard.as_mut().ok_or("No active A/B session")?;

    session.trim_db = trim_db;
    Ok(())
}

/// Build the tray menu with profiles
fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let profiles = list_profiles().unwrap_or_default();
    let current = app
        .state::<AppState>()
        .settings
        .lock()
        .ok()
        .and_then(|s| s.current_profile.clone());

    let mut items: Vec<MenuItem<tauri::Wry>> = Vec::new();

    // Add profile items
    for profile in profiles {
        let label = if Some(&profile) == current.as_ref() {
            format!("✓ {}", profile)
        } else {
            format!("   {}", profile)
        };

        let item = MenuItem::with_id(app, &profile, &label, true, None::<&str>)?;
        items.push(item);
    }

    // Build menu with profile items
    let menu = if items.is_empty() {
        let no_profiles =
            MenuItem::with_id(app, "no_profiles", "(No profiles)", false, None::<&str>)?;
        Menu::with_items(app, &[&no_profiles])?
    } else {
        // Create refs for menu
        let item_refs: Vec<&MenuItem<tauri::Wry>> = items.iter().collect();
        Menu::with_items(
            app,
            &item_refs
                .iter()
                .map(|i| *i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
                .collect::<Vec<_>>(),
        )?
    };

    // Add separator
    let separator = PredefinedMenuItem::separator(app)?;
    menu.append(&separator)?;

    // Add Show Window option
    let show_item = MenuItem::with_id(app, "show_window", "Show Window", true, None::<&str>)?;
    menu.append(&show_item)?;

    // Add Quit option
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    menu.append(&quit_item)?;

    Ok(menu)
}

/// Update tray menu (refresh profiles)
fn update_tray_menu(app: &AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main_tray") {
        let menu = build_tray_menu(app).map_err(|e| format!("Failed to build menu: {}", e))?;
        tray.set_menu(Some(menu))
            .map_err(|e| format!("Failed to set menu: {}", e))?;
    }
    Ok(())
}

/// Command to refresh tray menu (called from frontend when profiles change)
#[tauri::command]
fn refresh_tray_menu(app: AppHandle) -> Result<(), String> {
    update_tray_menu(&app)
}

/// Apply a profile by name (load and apply it)
fn apply_profile_by_name(app: &AppHandle, name: &str) -> Result<(), String> {
    // Load the profile
    let profile = load_profile(name.to_string())?;

    // Get current eq_enabled state
    let eq_enabled = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|s| s.eq_enabled)
        .unwrap_or(true);

    // Apply the profile
    apply_profile(
        profile.bands.clone(),
        profile.preamp,
        None,
        Some(eq_enabled),
    )?;

    // Update state and settings
    if let Ok(mut settings) = app.state::<AppState>().settings.lock() {
        settings.current_profile = Some(name.to_string());
        settings.bands = profile.bands;
        settings.preamp = profile.preamp;
        let _ = save_settings(&settings);
    }

    // Emit event to frontend
    let _ = app.emit("profile-changed-from-tray", name.to_string());

    // Update tray menu to show new selection
    let _ = update_tray_menu(app);

    Ok(())
}

/// Setup the system tray
fn setup_tray(app: &AppHandle) -> Result<(), tauri::Error> {
    let menu = build_tray_menu(app)?;

    let _tray = TrayIconBuilder::with_id("main_tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("EQAPO GUI")
        .on_menu_event(move |app, event| {
            let id = event.id.as_ref();
            match id {
                "show_window" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                "no_profiles" => {}
                profile_name => {
                    let _ = apply_profile_by_name(app, profile_name);
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ============================================================================
// Audio Monitor Commands
// ============================================================================

/// Get current audio output device information
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

/// Start peak meter monitoring
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

/// Stop peak meter monitoring
#[cfg(windows)]
#[tauri::command]
fn stop_peak_meter(state: tauri::State<AppState>) {
    state.audio_monitor.stop_peak_monitoring();
}

#[cfg(not(windows))]
#[tauri::command]
fn stop_peak_meter() {}

/// Get current peak value without starting monitoring
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load initial settings
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
        .invoke_handler(tauri::generate_handler![
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
            // A/B Test commands
            start_ab_session,
            apply_ab_option,
            record_ab_answer,
            get_ab_state,
            finish_ab_session,
            update_ab_trim,
            // Audio monitor commands
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
        // Note: q_factor is formatted as .2f so 0.707 becomes 0.71
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
        // Frequency should be cast to i32, truncating decimals
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
    fn app_settings_default_eq_enabled() {
        assert!(default_eq_enabled());
    }

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
