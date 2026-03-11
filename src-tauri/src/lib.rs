mod ssh_manager;
pub mod profile_storage;
pub mod settings_storage;
pub mod wsl_detector;
pub mod pty_manager;
mod sftp_manager;



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
            ssh_manager::connect_ssh,
            ssh_manager::write_stdin,
            ssh_manager::resize_pty,
            sftp_manager::sftp_list_dir,
            sftp_manager::sftp_download_file,
            sftp_manager::sftp_upload_file,
            profile_storage::load_profiles,
            profile_storage::save_profile,
            profile_storage::delete_profile,
            settings_storage::load_settings,
            settings_storage::save_settings,
            settings_storage::get_storage_path_info,
            settings_storage::set_config_directory,
            settings_storage::get_settings_path_info,
            settings_storage::load_autocomplete,
            settings_storage::save_autocomplete,
            wsl_detector::get_wsl_distributions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
