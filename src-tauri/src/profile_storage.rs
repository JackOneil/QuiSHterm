use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SshProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub private_key: Option<String>,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default = "default_terminal_type")]
    pub terminal_type: String,
}

fn default_terminal_type() -> String {
    "xterm-256color".to_string()
}

#[derive(Serialize, Deserialize, Default)]
pub struct ProfileStore {
    pub profiles: Vec<SshProfile>,
}

pub fn get_storage_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = crate::settings_storage::get_config_root(app);
    crate::settings_storage::ensure_config_root(&app_dir)?;

    Ok(app_dir.join("profiles.json"))
}

#[tauri::command]
pub fn load_profiles(app: AppHandle) -> Result<Vec<SshProfile>, String> {
    let path = get_storage_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;
    
    let store: ProfileStore = serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse profiles: {}", e))?;

    Ok(store.profiles)
}

#[tauri::command]
pub fn save_profile(app: AppHandle, profile: SshProfile) -> Result<Vec<SshProfile>, String> {
    let path = get_storage_path(&app)?;
    
    let mut store = if path.exists() {
        let data = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read profiles: {}", e))?;
        serde_json::from_str::<ProfileStore>(&data)
            .unwrap_or_default()
    } else {
        ProfileStore::default()
    };

    if let Some(existing) = store.profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile;
    } else {
        store.profiles.push(profile);
    }

    let serialized = serde_json::to_string_pretty(&store)
        .map_err(|e| format!("Failed to serialize profiles: {}", e))?;
    
    fs::write(&path, serialized)
        .map_err(|e| format!("Failed to write profiles: {}", e))?;

    Ok(store.profiles)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, id: String) -> Result<Vec<SshProfile>, String> {
    let path = get_storage_path(&app)?;
    
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;
    let mut store: ProfileStore = serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse profiles: {}", e))?;

    store.profiles.retain(|p| p.id != id);

    let serialized = serde_json::to_string_pretty(&store)
        .map_err(|e| format!("Failed to serialize profiles: {}", e))?;
    
    fs::write(&path, serialized)
        .map_err(|e| format!("Failed to write profiles: {}", e))?;

    Ok(store.profiles)
}
