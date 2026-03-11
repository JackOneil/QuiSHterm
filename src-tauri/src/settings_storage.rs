use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const SETTINGS_FILE_NAME: &str = "settings.json";
const AUTOCOMPLETE_FILE_NAME: &str = "autocomplete.json";
const STORAGE_LOCATION_FILE_NAME: &str = "storage-location.json";

#[derive(Serialize, Deserialize, Clone)]
struct StorageLocation {
    pub custom_config_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoragePathInfo {
    pub config_dir: String,
    pub default_config_dir: String,
    pub settings_file: String,
    pub is_custom: bool,
}

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
pub struct WindowStateSettings {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
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
    #[serde(default = "default_true")]
    pub enable_autocomplete: bool,
    #[serde(default)]
    pub window_state: Option<WindowStateSettings>,
}

fn default_scrollback() -> u32 {
    10000
}

fn default_true() -> bool {
    true
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
            enable_autocomplete: true,
            window_state: None,
        }
    }
}

fn get_default_config_dir(app: &AppHandle) -> PathBuf {
    app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn get_storage_location_path(app: &AppHandle) -> PathBuf {
    let mut path = get_default_config_dir(app);
    path.push(STORAGE_LOCATION_FILE_NAME);
    path
}

fn load_storage_location(app: &AppHandle) -> Option<PathBuf> {
    let locator_path = get_storage_location_path(app);
    let data = fs::read_to_string(locator_path).ok()?;
    let location = serde_json::from_str::<StorageLocation>(&data).ok()?;
    let custom_path = location.custom_config_dir?.trim().to_string();
    if custom_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(custom_path))
    }
}

pub fn get_config_root(app: &AppHandle) -> PathBuf {
    load_storage_location(app).unwrap_or_else(|| get_default_config_dir(app))
}

pub fn ensure_config_root(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Failed to create config directory: {}", e))
}

fn persist_storage_location(app: &AppHandle, custom_dir: Option<&Path>) -> Result<(), String> {
    let default_dir = get_default_config_dir(app);
    ensure_config_root(&default_dir)?;

    let locator_path = get_storage_location_path(app);
    if let Some(path) = custom_dir {
        let payload = StorageLocation {
            custom_config_dir: Some(path.to_string_lossy().to_string()),
        };
        let data = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        fs::write(locator_path, data).map_err(|e| format!("Failed to write storage locator: {}", e))?;
    } else if locator_path.exists() {
        fs::remove_file(locator_path).map_err(|e| format!("Failed to remove storage locator: {}", e))?;
    }

    Ok(())
}

fn migrate_config_files(from_root: &Path, to_root: &Path) -> Result<(), String> {
    if from_root == to_root {
        return Ok(());
    }

    ensure_config_root(to_root)?;

    for file_name in [SETTINGS_FILE_NAME, "profiles.json", AUTOCOMPLETE_FILE_NAME] {
        let from_path = from_root.join(file_name);
        let to_path = to_root.join(file_name);
        if from_path.exists() {
            fs::copy(&from_path, &to_path).map_err(|e| {
                format!(
                    "Failed to migrate {} from {} to {}: {}",
                    file_name,
                    from_path.to_string_lossy(),
                    to_path.to_string_lossy(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

pub fn get_settings_path(app: &AppHandle) -> PathBuf {
    let mut path = get_config_root(app);
    path.push(SETTINGS_FILE_NAME);
    path
}

#[tauri::command]
pub fn get_settings_path_info(app: AppHandle) -> String {
    let path = get_settings_path(&app);
    path.to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_storage_path_info(app: AppHandle) -> StoragePathInfo {
    let default_dir = get_default_config_dir(&app);
    let config_dir = get_config_root(&app);
    let settings_file = config_dir.join(SETTINGS_FILE_NAME);

    StoragePathInfo {
        config_dir: config_dir.to_string_lossy().to_string(),
        default_config_dir: default_dir.to_string_lossy().to_string(),
        settings_file: settings_file.to_string_lossy().to_string(),
        is_custom: config_dir != default_dir,
    }
}

#[tauri::command]
pub fn set_config_directory(app: AppHandle, directory: Option<String>) -> Result<StoragePathInfo, String> {
    let current_root = get_config_root(&app);
    let default_root = get_default_config_dir(&app);
    let target_root = match directory {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path.trim()),
        _ => default_root.clone(),
    };

    ensure_config_root(&target_root)?;
    migrate_config_files(&current_root, &target_root)?;

    if target_root == default_root {
        persist_storage_location(&app, None)?;
        log_debug(&app, &format!("Config directory reset to default: {}", target_root.to_string_lossy()));
    } else {
        persist_storage_location(&app, Some(&target_root))?;
        log_debug(&app, &format!("Config directory changed to: {}", target_root.to_string_lossy()));
    }

    Ok(get_storage_path_info(app))
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
        ensure_config_root(parent)?;
    }
    let data = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_window_state(app: AppHandle) -> Option<WindowStateSettings> {
    let settings = load_settings(app.clone());
    if let Some(window_state) = settings.window_state.clone() {
        log_debug(
            &app,
            &format!(
                "Loaded saved window state: {}x{}, maximized={}",
                window_state.width, window_state.height, window_state.maximized
            ),
        );
        Some(window_state)
    } else {
        log_debug(&app, "No saved window state found.");
        None
    }
}

#[tauri::command]
pub fn save_window_state(app: AppHandle, window_state: WindowStateSettings) -> Result<(), String> {
    log_debug(
        &app,
        &format!(
            "Saving window state: {}x{}, maximized={}",
            window_state.width, window_state.height, window_state.maximized
        ),
    );

    let mut settings = load_settings(app.clone());
    settings.window_state = Some(window_state);
    save_settings(app, settings)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AutocompleteDict {
    pub globals: Vec<String>,
    pub commands: HashMap<String, Vec<String>>,
}

impl Default for AutocompleteDict {
    fn default() -> Self {
        let mut commands = HashMap::new();
        commands.insert("systemctl".to_string(), vec!["start".to_string(), "stop".to_string(), "restart".to_string(), "status".to_string(), "enable".to_string(), "disable".to_string(), "reload".to_string()]);
        commands.insert("docker".to_string(), vec!["run".to_string(), "ps".to_string(), "build".to_string(), "images".to_string(), "rm".to_string(), "rmi".to_string(), "stop".to_string(), "start".to_string(), "exec".to_string(), "logs".to_string()]);
        commands.insert("docker-compose".to_string(), vec!["up".to_string(), "down".to_string(), "start".to_string(), "stop".to_string(), "logs".to_string(), "build".to_string()]);
        commands.insert("git".to_string(), vec!["status".to_string(), "add".to_string(), "commit".to_string(), "push".to_string(), "pull".to_string(), "checkout".to_string(), "branch".to_string(), "clone".to_string(), "merge".to_string(), "fetch".to_string(), "log".to_string()]);
        commands.insert("apt".to_string(), vec!["install".to_string(), "update".to_string(), "upgrade".to_string(), "remove".to_string(), "search".to_string()]);
        commands.insert("apt-get".to_string(), vec!["install".to_string(), "update".to_string(), "upgrade".to_string(), "remove".to_string()]);
        commands.insert("npm".to_string(), vec!["install".to_string(), "run".to_string(), "start".to_string(), "test".to_string(), "build".to_string(), "init".to_string()]);
        commands.insert("cargo".to_string(), vec!["build".to_string(), "run".to_string(), "test".to_string(), "check".to_string(), "add".to_string(), "publish".to_string()]);
        commands.insert("ls".to_string(), vec!["-l".to_string(), "-a".to_string(), "-la".to_string(), "-h".to_string(), "--help".to_string()]);

        let globals = vec![
            "cd", "pwd", "grep", "cat", "vim", "nano", "top", "htop", "sudo", 
            "dnf", "pacman", "ssh", "tar", "unzip", "curl", "wget", "find", "history", "clear", 
            "exit", "chown", "chmod", "rm", "mv", "cp", "mkdir", "rmdir", "touch", 
            "df", "du", "kill", "ps", "tail", "less", "awk", "sed", "python3", "node", 
            "rsync", "scp", "ping", "netstat", "ip", "ifconfig", "journalctl",
            "kubectl", "alias", "bash", "zsh"
        ].into_iter().map(|s| s.to_string()).collect();

        Self { globals, commands }
    }
}

pub fn get_autocomplete_path(app: &AppHandle) -> PathBuf {
    let mut path = get_config_root(app);
    path.push(AUTOCOMPLETE_FILE_NAME);
    path
}

#[tauri::command]
pub fn load_autocomplete(app: AppHandle) -> AutocompleteDict {
    let path = get_autocomplete_path(&app);
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(dict) = serde_json::from_str::<AutocompleteDict>(&data) {
            return dict;
        }
    }
    // If not exists or invalid, create default and persist it so user can edit later
    let default_dict = AutocompleteDict::default();
    let _ = save_autocomplete(app, default_dict.clone());
    default_dict
}

#[tauri::command]
pub fn save_autocomplete(app: AppHandle, dict: AutocompleteDict) -> Result<(), String> {
    let path = get_autocomplete_path(&app);
    if let Some(parent) = path.parent() {
        ensure_config_root(parent)?;
    }
    let data = serde_json::to_string_pretty(&dict).map_err(|e| e.to_string())?;
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
