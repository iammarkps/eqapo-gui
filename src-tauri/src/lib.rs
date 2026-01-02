use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

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
        }
    }
}

/// Managed state for tracking current settings
pub struct AppState {
    pub settings: Mutex<AppSettings>,
}

/// Get the AntigravityEQ directory in Documents
fn get_app_dir() -> Result<PathBuf, String> {
    let docs = dirs::document_dir().ok_or("Could not find Documents folder")?;
    Ok(docs.join("AntigravityEQ"))
}

/// Ensure all required directories exist
fn ensure_dirs() -> Result<PathBuf, String> {
    let app_dir = get_app_dir()?;
    let profiles_dir = app_dir.join("profiles");

    fs::create_dir_all(&profiles_dir)
        .map_err(|e| format!("Failed to create directories: {}", e))?;

    Ok(app_dir)
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
) -> Result<(), String> {
    let target_path = if let Some(path) = config_path {
        PathBuf::from(path)
    } else {
        let app_dir = ensure_dirs()?;
        app_dir.join("live_config.txt")
    };

    // Build EqualizerAPO config content
    let mut lines = vec![
        String::from("; AntigravityEQ Live Configuration"),
        String::from("; Auto-generated - do not edit manually"),
        String::from(""),
        format!("Preamp: {:.1} dB", preamp),
        String::from(""),
    ];

    for band in &bands {
        lines.push(band.to_eapo_line());
    }

    let content = lines.join("\r\n");

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
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = std::process::Command::new("icacls")
                .arg(&target_path)
                .arg("/grant")
                .arg("Everyone:F") // Try granting Full Control
                .creation_flags(CREATE_NO_WINDOW)
                .output();
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
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let _ = std::process::Command::new("icacls")
            .arg(&target_path)
            .arg("/grant")
            .arg("Everyone:R")
            .creation_flags(CREATE_NO_WINDOW)
            .output();
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
    state: tauri::State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if let Ok(mut settings) = state.settings.lock() {
        settings.bands = bands;
        settings.preamp = preamp;
        settings.current_profile = current_profile;
        settings.config_path = config_path;
        save_settings(&settings)?;
    }

    // Update tray menu in case profile changed
    let _ = update_tray_menu(&app);

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

    // Apply the profile
    apply_profile(profile.bands.clone(), profile.preamp, None)?;

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
        .tooltip("AntigravityEQ")
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
            refresh_tray_menu
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
