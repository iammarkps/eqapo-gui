//! System tray handling for EQAPO GUI.
//!
//! This module manages the system tray icon, context menu, and profile switching
//! from the tray. It allows users to quickly switch between EQ profiles without
//! opening the main window.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::profile::{apply_profile, list_profiles, load_profile, save_settings};
use crate::types::AppState;

/// Builds the tray menu with available profiles.
///
/// Creates a context menu showing all available profiles with a checkmark
/// next to the currently active one. Also includes "Show Window" and "Quit" options.
///
/// # Arguments
///
/// * `app` - The Tauri app handle
///
/// # Returns
///
/// A `Menu` ready to be attached to the tray icon.
///
/// # Errors
///
/// Returns an error if menu items cannot be created.
fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let profiles = list_profiles().unwrap_or_default();
    let current = app
        .state::<AppState>()
        .settings
        .lock()
        .ok()
        .and_then(|s| s.current_profile.clone());

    let mut items: Vec<MenuItem<tauri::Wry>> = Vec::new();

    // Add profile items with checkmark for current
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
        let item_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
            items.iter().map(|i| i as _).collect();
        Menu::with_items(app, &item_refs)?
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

/// Updates the tray menu to reflect current profiles.
///
/// Should be called whenever the profile list or current profile changes
/// (e.g., after saving, deleting, or switching profiles).
///
/// # Arguments
///
/// * `app` - The Tauri app handle
///
/// # Errors
///
/// Returns an error if the menu cannot be built or set.
pub fn update_tray_menu(app: &AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main_tray") {
        let menu = build_tray_menu(app).map_err(|e| format!("Failed to build menu: {}", e))?;
        tray.set_menu(Some(menu))
            .map_err(|e| format!("Failed to set menu: {}", e))?;
    }
    Ok(())
}

/// Tauri command to refresh the tray menu.
///
/// Called from the frontend when profiles are added or deleted
/// to keep the tray menu in sync.
///
/// # Arguments
///
/// * `app` - The Tauri app handle
///
/// # Errors
///
/// Returns an error if the menu cannot be updated.
#[tauri::command]
pub fn refresh_tray_menu(app: AppHandle) -> Result<(), String> {
    update_tray_menu(&app)
}

/// Applies a profile by name when selected from the tray menu.
///
/// Loads the profile, applies it to EqualizerAPO, updates the application
/// state, persists settings, and notifies the frontend of the change.
///
/// # Arguments
///
/// * `app` - The Tauri app handle
/// * `name` - The name of the profile to apply
///
/// # Errors
///
/// Returns an error if the profile cannot be loaded or applied.
fn apply_profile_by_name(app: &AppHandle, name: &str) -> Result<(), String> {
    let profile = load_profile(name.to_string())?;

    let eq_enabled = app
        .state::<AppState>()
        .settings
        .lock()
        .map(|s| s.eq_enabled)
        .unwrap_or(true);

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

    // Emit event to frontend so it can sync its state
    let _ = app.emit("profile-changed-from-tray", name.to_string());

    // Update tray menu to show new selection
    let _ = update_tray_menu(app);

    Ok(())
}

/// Sets up the system tray icon and menu.
///
/// Creates the tray icon with:
/// - The application icon
/// - A context menu with profiles and actions
/// - Click handlers for menu items and tray icon
///
/// # Behavior
///
/// - Left-click on tray icon: Shows and focuses the main window
/// - Right-click on tray icon: Opens the context menu
/// - Menu profile item: Applies that profile
/// - "Show Window": Shows and focuses the main window
/// - "Quit": Exits the application
///
/// # Arguments
///
/// * `app` - The Tauri app handle
///
/// # Errors
///
/// Returns an error if the tray icon cannot be created.
pub fn setup_tray(app: &AppHandle) -> Result<(), tauri::Error> {
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
            // Left-click shows the window
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
