use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use tokio::sync::mpsc;
use std::thread;
use tauri::{AppHandle, Emitter};

use crate::ssh_manager::{SshAction, SshOutputPayload, SshState};
use crate::settings_storage::log_debug;

pub async fn connect_wsl(
    app_handle: AppHandle,
    state: tauri::State<'_, SshState>,
    session_id: String,
    distro_name: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    log_debug(&app_handle, &format!("Attempting local PTY WSL connection to {}", distro_name));

    // Create the PTY system
    let pty_system = NativePtySystem::default();

    // Create a new PTY pair
    let pair = pty_system.openpty(PtySize {
        rows: rows as u16,
        cols: cols as u16,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("Failed to open PTY: {}", e))?;

    let mut cmd = CommandBuilder::new("wsl.exe");
    cmd.arg("-d");
    cmd.arg(&distro_name);
    // You could also add `~` or equivalent if needed

    // Spawn the command inside the PTY
    let child = pair.slave.spawn_command(cmd).map_err(|e| format!("Failed to spawn wsl: {}", e))?;
    
    // Setup channels for writing to PTY
    let (tx_action, mut rx_action) = mpsc::channel::<SshAction>(32);
    let mut map = state.writers.lock().await;
    map.insert(session_id.clone(), tx_action);

    // Keep handles for reading and writing
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let sid_for_read = session_id.clone();
    let app_handle_read = app_handle.clone();

    // Dedicated read thread (blocking read strategy similar to SSH)
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut utf8_remainder: Vec<u8> = Vec::new();
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    // Prepend any leftover bytes from the previous read
                    let mut combined = std::mem::take(&mut utf8_remainder);
                    combined.extend_from_slice(&buf[..n]);
                    
                    // Find the last valid UTF-8 boundary
                    let valid_up_to = match std::str::from_utf8(&combined) {
                        Ok(_) => combined.len(),
                        Err(e) => {
                            if e.error_len().is_none() {
                                e.valid_up_to()
                            } else {
                                e.valid_up_to() + e.error_len().unwrap()
                            }
                        }
                    };
                    
                    if valid_up_to > 0 {
                        let text = unsafe { std::str::from_utf8_unchecked(&combined[..valid_up_to]) };
                        let _ = app_handle_read.emit("ssh-output", SshOutputPayload {
                            session_id: sid_for_read.clone(),
                            data: text.to_string(),
                        });
                    }
                    
                    if valid_up_to < combined.len() {
                        utf8_remainder = combined[valid_up_to..].to_vec();
                    }
                }
                Err(e) => {
                    log_debug(&app_handle_read, &format!("PTY read error: {:?}", e));
                    break;
                }
            }
        }
        let _ = app_handle_read.emit("ssh-terminated", sid_for_read);
    });

    let sid_for_write = session_id.clone();
    let app_handle_write = app_handle.clone();
    let master_handle = pair.master;

    // Dedicated Tokio task for writing and resizing
    tokio::spawn(async move {
        while let Some(action) = rx_action.recv().await {
            match action {
                SshAction::Write(data) => {
                    if let Err(e) = writer.write_all(&data) {
                        log_debug(&app_handle_write, &format!("PTY write error: {:?}", e));
                        break;
                    }
                    let _ = writer.flush();
                }
                SshAction::Resize(cols, rows) => {
                    if let Err(e) = master_handle.resize(PtySize {
                        cols: cols as u16,
                        rows: rows as u16,
                        pixel_width: 0,
                        pixel_height: 0,
                    }) {
                        log_debug(&app_handle_write, &format!("PTY resize error: {:?}", e));
                    }
                }
            }
        }
        
        log_debug(&app_handle_write, &format!("Closed PTY action loop for {}", sid_for_write));
        // Cleanup child process when write queue closes or error occurs
        drop(child);
    });

    Ok(())
}
