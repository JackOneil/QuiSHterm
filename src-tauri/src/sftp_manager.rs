use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crate::ssh_manager::get_sftp_session;

#[derive(Serialize, Deserialize, Debug)]
pub struct SftpFile {
    name: String,
    is_dir: bool,
    size: u64,
    modified: u64,
}

#[tauri::command]
pub async fn sftp_list_dir(session_id: String, path: String) -> Result<Vec<SftpFile>, String> {
    let arc_sess: std::sync::Arc<std::sync::Mutex<ssh2::Session>> = get_sftp_session(&session_id).await
        .ok_or_else(|| "SSH session not found".to_string())?;
        
    let sess = arc_sess.lock().map_err(|_| "Failed to lock SSH session")?;
    let sftp = sess.sftp().map_err(|e| format!("Failed to initiate SFTP: {}", e))?;
    
    let stats = sftp.readdir(std::path::Path::new(&path))
        .map_err(|e| format!("Failed to read directory: {}", e))?;
        
    let mut files: Vec<SftpFile> = stats.into_iter().map(|(path_buf, stat): (std::path::PathBuf, ssh2::FileStat)| {
        let name = path_buf.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
            
        SftpFile {
            name,
            is_dir: stat.is_dir(),
            size: stat.size.unwrap_or(0),
            modified: stat.mtime.unwrap_or(0),
        }
    }).collect();
    
    // Sort directories first, then alphabetical
    files.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    
    Ok(files)
}

#[tauri::command]
pub async fn sftp_download_file(session_id: String, remote_path: String) -> Result<String, String> {
    let arc_sess: std::sync::Arc<std::sync::Mutex<ssh2::Session>> = get_sftp_session(&session_id).await
        .ok_or_else(|| "SSH session not found".to_string())?;
        
    let sess = arc_sess.lock().map_err(|_| "Failed to lock SSH session")?;
    let sftp = sess.sftp().map_err(|e| format!("Failed to initiate SFTP: {}", e))?;
    
    let mut file = sftp.open(std::path::Path::new(&remote_path))
        .map_err(|e| format!("Failed to open remote file: {}", e))?;
        
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read file: {}", e))?;
        
    // Return to UI as base64 string
    Ok(BASE64.encode(buffer))
}

#[tauri::command]
pub async fn sftp_upload_file(session_id: String, remote_path: String, base64_data: String) -> Result<(), String> {
    let arc_sess: std::sync::Arc<std::sync::Mutex<ssh2::Session>> = get_sftp_session(&session_id).await
        .ok_or_else(|| "SSH session not found".to_string())?;
        
    let data = BASE64.decode(base64_data)
        .map_err(|e| format!("Failed to decode base64 input: {}", e))?;
        
    let sess = arc_sess.lock().map_err(|_| "Failed to lock SSH session")?;
    let sftp = sess.sftp().map_err(|e| format!("Failed to initiate SFTP: {}", e))?;
    
    let mut file = sftp.create(std::path::Path::new(&remote_path))
        .map_err(|e| format!("Failed to create remote file: {}", e))?;
        
    file.write_all(&data)
        .map_err(|e| format!("Failed to write file: {}", e))?;
        
    Ok(())
}
