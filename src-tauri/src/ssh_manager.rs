use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use std::sync::OnceLock;
use crate::settings_storage::log_debug;
use tokio::sync::{mpsc, Mutex, RwLock};
use std::io::{Read, Write};
use ssh2::Session;

static SESSIONS: OnceLock<RwLock<HashMap<String, Arc<std::sync::Mutex<ssh2::Session>>>>> = OnceLock::new();

pub fn get_sessions() -> &'static RwLock<HashMap<String, Arc<std::sync::Mutex<ssh2::Session>>>> {
    SESSIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn get_sftp_session(session_id: &str) -> Option<Arc<std::sync::Mutex<ssh2::Session>>> {
    let map = get_sessions().read().await;
    map.get(session_id).cloned()
}

type WriterMap = Arc<Mutex<HashMap<String, mpsc::Sender<SshAction>>>>;

pub enum SshAction {
    Write(Vec<u8>),
    Resize(u32, u32),
}

pub struct SshState {
    pub writers: WriterMap,
}

impl SshState {
    pub fn new() -> Self {
        Self {
            writers: Arc::new(Mutex::new(HashMap::<String, mpsc::Sender<SshAction>>::new())),
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
    auth_type: Option<String>,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    // WSL Routing
    if let Some(auth) = auth_type.as_ref() {
        if auth == "wsl" {
            // Using `host` as the distro name from the frontend setup
            return crate::pty_manager::connect_wsl(app.clone(), state.clone(), session_id, host, cols, rows).await;
        }
    }

    let app_handle = app.clone();
    let writers = state.writers.clone();
    
    let (tx_result, rx_result) = tokio::sync::oneshot::channel();

    // Zabalíme celou SSH logiku do blokujícího threadu
    tokio::task::spawn_blocking(move || {
        let connect_logic = || -> Result<(Session, ssh2::Channel), String> {
            log_debug(&app_handle, &format!("Connecting to {}:{}", host, port));
            let tcp = TcpStream::connect(format!("{}:{}", host, port)).map_err(|e| format!("Connect error: {}", e))?;
            let mut sess = Session::new().map_err(|e: ssh2::Error| e.to_string())?;
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
            let mut channel = sess.channel_session().map_err(|e: ssh2::Error| e.to_string())?;
            channel.request_pty("xterm-256color", None, Some((cols, rows, 0, 0))).map_err(|e| format!("PTY error: {}", e))?;
            channel.shell().map_err(|e| format!("Shell error: {}", e))?;
            log_debug(&app_handle, "Shell requested successfully.");
            
            Ok((sess, channel))
        };

        match connect_logic() {
            Ok((sess, mut channel)) => {
                let _ = tx_result.send(Ok(()));
                
                // Store the SSH session globally for the SFTP Manager to spawn sub-channels
                let arc_sess = Arc::new(std::sync::Mutex::new(sess));
                
                let arc_sess_clone_for_map: std::sync::Arc<std::sync::Mutex<ssh2::Session>> = arc_sess.clone();
                let arc_session_id_clone_for_map: String = session_id.clone();
                
                // Vložíme Arc resferenci do globální mapy pro další použití
                tokio::spawn(async move {
                    get_sessions().write().await.insert(arc_session_id_clone_for_map, arc_sess_clone_for_map);
                });
                
                // Pro kanál mezi UI a blokujícím vláknem 
                let (tx, mut rx) = mpsc::channel::<SshAction>(32);
                let sid_for_write = session_id.clone();
                let writers_clone = writers.clone();
                
                tokio::spawn(async move {
                    let mut map = writers_clone.lock().await;
                    map.insert(sid_for_write, tx);
                });

                // Oddělíme čtení a zápis
                let mut stream = channel.stream(0);
                let sid_for_task = session_id.clone();
                let mut rx_bytes: u64 = 0;
                let mut tx_bytes: u64 = 0;
                let mut last_stats_emit = std::time::Instant::now();
                let mut utf8_remainder: Vec<u8> = Vec::new();
                
                // Převedeme mut rx tokio receiver do blokučního
                loop {
                    let mut buf = [0; 4096];
                    
                    // Bezpečně uzamkneme session skrz Arc
                    let read_res = if let Ok(mut locked_sess) = arc_sess.lock() {
                        locked_sess.set_timeout(50); // Mírný timeout v ms pro čtení
                        stream.read(&mut buf)
                    } else {
                        break; // Deadlock
                    };
                    
                    match read_res {
                        Ok(0) => {
                            log_debug(&app_handle, "SSH stream reached EOF");
                            break;
                        }
                        Ok(n) => {
                            rx_bytes += n as u64;
                            
                            // Prepend any leftover bytes from the previous read
                            let mut combined = std::mem::take(&mut utf8_remainder);
                            combined.extend_from_slice(&buf[..n]);
                            
                            // Find the last valid UTF-8 boundary
                            let valid_up_to = match std::str::from_utf8(&combined) {
                                Ok(_) => combined.len(), // All bytes are valid UTF-8
                                Err(e) => {
                                    let valid = e.valid_up_to();
                                    // Check if the error is at the end (incomplete sequence)
                                    // vs in the middle (truly invalid byte)
                                    if e.error_len().is_none() {
                                        // Incomplete multi-byte sequence at the end
                                        valid
                                    } else {
                                        // Truly invalid byte — skip it to avoid infinite loop
                                        valid + e.error_len().unwrap()
                                    }
                                }
                            };
                            
                            if valid_up_to > 0 {
                                // SAFETY: we just verified this range is valid UTF-8
                                let text = unsafe { std::str::from_utf8_unchecked(&combined[..valid_up_to]) };
                                let _ = app_handle.emit("ssh-output", SshOutputPayload {
                                    session_id: sid_for_task.clone(),
                                    data: text.to_string(),
                                });
                            }
                            
                            // Store any trailing incomplete bytes for the next read
                            if valid_up_to < combined.len() {
                                utf8_remainder = combined[valid_up_to..].to_vec();
                            }
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
                        if let Ok(mut locked_sess) = arc_sess.lock() {
                            locked_sess.set_timeout(0); // Zruš timeout
                        }
                        
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
                        if let Ok(mut locked_sess) = arc_sess.lock() {
                            locked_sess.set_timeout(50); // Obnov timeout
                        }
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

                let sid_cleanup = session_id.clone();
                let writers_clone2 = writers.clone();
                tokio::spawn(async move {
                    let mut wc = writers_clone2.lock().await;
                    wc.remove(&sid_cleanup);
                    get_sessions().write().await.remove(&sid_cleanup);
                });
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
    let map = state.writers.lock().await;
    if let Some(writer) = map.get(&session_id) {
        writer.send(SshAction::Write(data.into_bytes())).await.map_err(|e: tokio::sync::mpsc::error::SendError<SshAction>| e.to_string())?;
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
    let map = state.writers.lock().await;
    if let Some(writer) = map.get(&session_id) {
        writer.send(SshAction::Resize(cols, rows)).await.map_err(|e: tokio::sync::mpsc::error::SendError<SshAction>| e.to_string())?;
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}
