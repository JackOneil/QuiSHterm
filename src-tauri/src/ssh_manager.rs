use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use std::io::{Read, Write};
use tauri::{AppHandle, Emitter};
use ssh2::Session;
use crate::settings_storage::log_debug;

pub enum SshAction {
    Write(Vec<u8>),
    Resize(u32, u32),
}

pub struct SshState {
    pub writers: Arc<Mutex<HashMap<String, mpsc::Sender<SshAction>>>>,
}

impl SshState {
    pub fn new() -> Self {
        Self {
            writers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(serde::Serialize, Clone)]
pub struct SshOutputPayload {
    pub session_id: String,
    pub data: String,
}

#[derive(serde::Serialize, Clone)]
pub struct SshStatsPayload {
    pub session_id: String,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
}

#[tauri::command]
pub async fn connect_ssh(
    app: AppHandle,
    state: tauri::State<'_, SshState>,
    session_id: String,
    host: String,
    port: u16,
    user: String,
    password: Option<String>,
    private_key: Option<String>,
) -> Result<(), String> {
    let app_handle = app.clone();
    let writers = state.writers.clone();
    
    let (tx_result, rx_result) = tokio::sync::oneshot::channel();

    // Zabalíme celou SSH logiku do blokujícího threadu
    tokio::task::spawn_blocking(move || {
        let connect_logic = || -> Result<(Session, ssh2::Channel), String> {
            log_debug(&app_handle, &format!("Connecting to {}:{}", host, port));
            let tcp = TcpStream::connect(format!("{}:{}", host, port)).map_err(|e| format!("Connect error: {}", e))?;
            let mut sess = Session::new().map_err(|e| e.to_string())?;
            sess.set_tcp_stream(tcp);
            log_debug(&app_handle, "Starting handshake");
            sess.handshake().map_err(|e| format!("Handshake error: {}", e))?;

            if let Some(key_path_str) = private_key.filter(|k| !k.trim().is_empty()) {
                log_debug(&app_handle, &format!("Attempting public key auth with: {}", key_path_str));
                let path = if key_path_str.starts_with("~/") {
                    let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("~"));
                    p.push(key_path_str.trim_start_matches("~/"));
                    p
                } else {
                    std::path::PathBuf::from(&key_path_str)
                };

                let pass_ref = password.as_deref().filter(|p| !p.trim().is_empty());
                sess.userauth_pubkey_file(&user, None, &path, pass_ref)
                    .map_err(|e| format!("Pubkey Auth error: {}", e))?;
            } else if let Some(pass) = password.filter(|p| !p.trim().is_empty()) {
                log_debug(&app_handle, "Attempting password auth");
                sess.userauth_password(&user, &pass).map_err(|e| format!("Password Auth error: {}", e))?;
            } else {
                return Err("Password or Private Key required".to_string());
            }

            if !sess.authenticated() {
                return Err("Authentication failed".to_string());
            }

            log_debug(&app_handle, "Authenticated! Requesting PTY...");
            let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
            channel.request_pty("xterm", None, Some((80, 24, 0, 0))).map_err(|e| format!("PTY error: {}", e))?;
            channel.shell().map_err(|e| format!("Shell error: {}", e))?;
            log_debug(&app_handle, "Shell requested successfully.");
            
            Ok((sess, channel))
        };

        match connect_logic() {
            Ok((mut sess, mut channel)) => {
                let _ = tx_result.send(Ok(()));
                
                // Pro kanál mezi UI a blokujícím vláknem 
                let (tx, mut rx) = mpsc::channel::<SshAction>(32);
                let sid_for_write = session_id.clone();
                let writers_clone = writers.clone();
                
                tokio::spawn(async move {
                    writers_clone.lock().await.insert(sid_for_write, tx);
                });

                // Oddělíme čtení a zápis
                let mut stream = channel.stream(0);
                let sid_for_task = session_id.clone();
                let mut rx_bytes: u64 = 0;
                let mut tx_bytes: u64 = 0;
                let mut last_stats_emit = std::time::Instant::now();
                
                // Převedeme mut rx tokio receiver do blokučního
                loop {
                    let mut buf = [0; 4096];
                    sess.set_timeout(50); // Mírný timeout v ms pro čtení
                    
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            log_debug(&app_handle, "SSH stream reached EOF");
                            break;
                        }
                        Ok(n) => {
                            rx_bytes += n as u64;
                            let _ = app_handle.emit("ssh-output", SshOutputPayload {
                                session_id: sid_for_task.clone(),
                                data: String::from_utf8_lossy(&buf[..n]).to_string(),
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                            // Timeout passed without receiving new data
                        }
                        Err(e) => {
                            log_debug(&app_handle, &format!("SSH read error: {:?}", e));
                            break;
                        }
                    }

                    // Vyčti frontu zápisů došlých z Tauri UI
                    while let Ok(action) = rx.try_recv() {
                        sess.set_timeout(0); // Zruš timeout
                        match action {
                            SshAction::Write(data) => {
                                if let Err(e) = stream.write_all(&data) {
                                    log_debug(&app_handle, &format!("SSH write_all error: {:?}", e));
                                    break;
                                }
                                tx_bytes += data.len() as u64;
                                let _ = stream.flush();
                            }
                            SshAction::Resize(cols, rows) => {
                                if let Err(e) = channel.request_pty_size(cols, rows, None, None) {
                                    log_debug(&app_handle, &format!("SSH pty resize error: {:?}", e));
                                }
                            }
                        }
                        sess.set_timeout(50); // Obnov timeout
                    }
                    
                    // Emit stats periodically (every 1 second max) to avoid flooding the frontend
                    if last_stats_emit.elapsed().as_millis() >= 1000 {
                        let _ = app_handle.emit("ssh-stats", SshStatsPayload {
                            session_id: sid_for_task.clone(),
                            tx_bytes,
                            rx_bytes,
                        });
                        last_stats_emit = std::time::Instant::now();
                    }
                }

                log_debug(&app_handle, &format!("Loop ended, closing session {:?}", session_id));
                let _ = app_handle.emit("ssh-terminated", session_id.clone());
            }
            Err(e) => {
                log_debug(&app_handle, &format!("Connection Setup Failed: {}", e));
                let _ = tx_result.send(Err(e));
            }
        }
    });

    rx_result.await.unwrap_or(Err("Internal spawn error".to_string()))
}


#[tauri::command]
pub async fn write_stdin(
    state: tauri::State<'_, SshState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    if let Some(writer) = state.writers.lock().await.get(&session_id) {
        writer.send(SshAction::Write(data.into_bytes())).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}

#[tauri::command]
pub async fn resize_pty(
    state: tauri::State<'_, SshState>,
    session_id: String,
    rows: u32,
    cols: u32,
) -> Result<(), String> {
    if let Some(writer) = state.writers.lock().await.get(&session_id) {
        writer.send(SshAction::Resize(cols, rows)).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}
