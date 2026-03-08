use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
pub struct HighlightRule {
    pub keyword: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConnectionFolder {
    pub id: String,
    pub name: String,
    pub color: String,
    pub profile_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub debug_mode: bool,
    pub highlights: Vec<HighlightRule>,
    #[serde(default)]
    pub folders: Vec<ConnectionFolder>,
    #[serde(default = "default_scrollback")]
    pub scrollback: u32,
    #[serde(default)]
    pub show_line_numbers: bool,
}

fn default_scrollback() -> u32 {
    10000
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            debug_mode: false,
            highlights: vec![
                HighlightRule { keyword: "ERROR".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "FAIL".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "FAILED".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "Invalid".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "Timeout".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "closed".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "denied".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "refused".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "fatal".to_string(), color: "red".to_string() },
                HighlightRule { keyword: "SUCCESS".to_string(), color: "green".to_string() },
                HighlightRule { keyword: "OK".to_string(), color: "green".to_string() },
                HighlightRule { keyword: "DONE".to_string(), color: "green".to_string() },
                HighlightRule { keyword: "active".to_string(), color: "green".to_string() },
                HighlightRule { keyword: "enabled".to_string(), color: "green".to_string() },
                HighlightRule { keyword: "running".to_string(), color: "green".to_string() },
                HighlightRule { keyword: "WARNING".to_string(), color: "yellow".to_string() },
                HighlightRule { keyword: "Warning".to_string(), color: "yellow".to_string() },
                HighlightRule { keyword: "deprecated".to_string(), color: "yellow".to_string() },
                HighlightRule { keyword: "INFO".to_string(), color: "blue".to_string() },
                HighlightRule { keyword: "Info".to_string(), color: "blue".to_string() },
                // Timestamps (ISO 8601, syslog, common formats)
                HighlightRule { keyword: r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[\.\d+]*[Z]?[\+\-\d{2}:\d{2}]*".to_string(), color: "cyan".to_string() },
                HighlightRule { keyword: r"\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}".to_string(), color: "cyan".to_string() },
                // IPv4 addresses
                HighlightRule { keyword: r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}".to_string(), color: "cyan".to_string() },
            ],
            folders: vec![],
            scrollback: 10000,
            show_line_numbers: false,
        }
    }
}

pub fn get_settings_path(app: &AppHandle) -> PathBuf {
    let mut path = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.push("settings.json");
    path
}

#[tauri::command]
pub fn get_settings_path_info(app: AppHandle) -> String {
    let path = get_settings_path(&app);
    path.to_string_lossy().to_string()
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> AppSettings {
    let path = get_settings_path(&app);
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(mut settings) = serde_json::from_str::<AppSettings>(&data) {
            // Merge: append any default highlights that are missing from saved settings
            let defaults = AppSettings::default();
            let existing_keywords: std::collections::HashSet<String> =
                settings.highlights.iter().map(|h| h.keyword.clone()).collect();
            for rule in defaults.highlights {
                if !existing_keywords.contains(&rule.keyword) {
                    settings.highlights.push(rule);
                }
            }
            return settings;
        }
    }
    AppSettings::default()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let path = get_settings_path(&app);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

pub fn log_debug(app: &AppHandle, message: &str) {
    let settings = load_settings(app.clone());
    if settings.debug_mode {
        if let Ok(mut exe_path) = std::env::current_exe() {
            exe_path.set_extension("log");
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(exe_path) {
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "[{}] [DEBUG] {}", timestamp, message);
            }
        }
    }
}
