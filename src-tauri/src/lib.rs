use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

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
fn apply_profile(bands: Vec<ParametricBand>, preamp: f32, config_path: Option<String>) -> Result<(), String> {
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
        fs::write(&target_path, &content)
            .map_err(|retry_err| format!("Failed to write to {:?}: {} (Retry: {})", target_path, e, retry_err))?;
    }

    // Fix permissions for EqualizerAPO (Windows Audio Service needs access)
    // We use icacls to grant "Everyone" read access
    #[cfg(target_os = "windows")]
    {
        // ... (standard read grant)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            load_profile,
            save_profile,
            apply_profile,
            delete_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
