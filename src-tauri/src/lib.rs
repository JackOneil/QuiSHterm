mod ssh_manager;
pub mod profile_storage;
pub mod settings_storage;
pub mod wsl_detector;
pub mod pty_manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            settings_storage::log_debug(app.handle(), "Application initialized successfully.");
            Ok(())
        })
        .manage(ssh_manager::SshState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            ssh_manager::connect_ssh,
            ssh_manager::write_stdin,
            ssh_manager::resize_pty,
            profile_storage::load_profiles,
            profile_storage::save_profile,
            profile_storage::delete_profile,
            settings_storage::load_settings,
            settings_storage::save_settings,
            settings_storage::get_settings_path_info,
            wsl_detector::get_wsl_distributions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
