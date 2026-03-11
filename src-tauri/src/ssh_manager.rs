use std::collections::HashMap;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use std::sync::OnceLock;
use crate::settings_storage::log_debug;
use tokio::sync::{mpsc, Mutex, RwLock};
use std::io::{Read, Write};
use ssh2::Session;

const PASSWORD_REQUIRED_CODE: &str = "PASSWORD_REQUIRED";
const PASSWORD_AUTH_FAILED_CODE: &str = "PASSWORD_AUTH_FAILED";

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

fn resolve_private_key_path(key_path_str: &str) -> PathBuf {
    if key_path_str.starts_with("~/") {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        path.push(key_path_str.trim_start_matches("~/"));
        path
    } else {
        PathBuf::from(key_path_str)
    }
}

fn default_key_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let Some(home_dir) = dirs::home_dir() else {
        return candidates;
    };

    let ssh_dir = home_dir.join(".ssh");
    for name in [
        "id_ed25519",
        "id_ecdsa",
        "id_ecdsa_sk",
        "id_rsa",
        "id_dsa",
        "id_xmss",
        "identity",
    ] {
        let path = ssh_dir.join(name);
        if path.exists() {
            candidates.push(path);
        }
    }

    candidates
}

fn try_agent_auth(sess: &Session, user: &str, app: &AppHandle) -> bool {
    #[cfg(target_os = "windows")]
    log_debug(app, "Attempting Pageant/SSH agent authentication.");
    #[cfg(not(target_os = "windows"))]
    log_debug(app, "Attempting SSH agent authentication.");

    let mut agent = match sess.agent() {
        Ok(agent) => agent,
        Err(e) => {
            log_debug(app, &format!("SSH agent unavailable: {}", e));
            return false;
        }
    };

    if let Err(e) = agent.connect() {
        log_debug(app, &format!("SSH agent connect failed: {}", e));
        return false;
    }

    if let Err(e) = agent.list_identities() {
        log_debug(app, &format!("SSH agent identity listing failed: {}", e));
        return false;
    }

    let identities = match agent.identities() {
        Ok(identities) => identities,
        Err(e) => {
            log_debug(app, &format!("SSH agent identities could not be read: {}", e));
            return false;
        }
    };

    if identities.is_empty() {
        log_debug(app, "SSH agent returned no identities.");
        return false;
    }

    for (index, identity) in identities.iter().enumerate() {
        log_debug(app, &format!("Trying SSH agent identity #{}.", index + 1));
        match agent.userauth(user, identity) {
            Ok(_) if sess.authenticated() => {
                log_debug(app, &format!("SSH agent authentication succeeded with identity #{}.", index + 1));
                return true;
            }
            Ok(_) => {
                log_debug(app, &format!("SSH agent identity #{} returned without authenticating.", index + 1));
            }
            Err(e) => {
                log_debug(app, &format!("SSH agent identity #{} failed: {}", index + 1, e));
            }
        }
    }

    false
}

fn try_pubkey_auth(sess: &Session, user: &str, key_path: &Path, passphrase: Option<&str>, app: &AppHandle, source: &str) -> bool {
    log_debug(app, &format!("Attempting public key authentication using {}: {}", source, key_path.to_string_lossy()));

    match sess.userauth_pubkey_file(user, None, key_path, passphrase) {
        Ok(_) if sess.authenticated() => {
            log_debug(app, &format!("Public key authentication succeeded using {}.", source));
            true
        }
        Ok(_) => {
            log_debug(app, &format!("Public key authentication using {} returned without authenticating.", source));
            false
        }
        Err(e) => {
            log_debug(app, &format!("Public key authentication using {} failed: {}", source, e));
            false
        }
    }
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
    terminal_type: Option<String>,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    // WSL Routing
    if let Some(auth) = auth_type.as_ref() {
        if auth == "wsl" {
            // Using `host` as the distro name from the frontend setup
            return crate::pty_manager::connect_wsl(app.clone(), state.clone(), session_id, host, terminal_type.clone(), cols, rows).await;
        }
    }

    let app_handle = app.clone();
    let writers = state.writers.clone();
    
    let (tx_result, rx_result) = tokio::sync::oneshot::channel();

    // Zabalíme celou SSH logiku do blokujícího threadu
    tokio::task::spawn_blocking(move || {
        let connect_logic = || -> Result<(Session, ssh2::Channel), String> {
            let auth_password = password.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);
            let passphrase = auth_password.as_deref();
            let requested_terminal = terminal_type
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("xterm-256color")
                .to_string();

            log_debug(&app_handle, &format!("Connecting to {}:{}", host, port));
            let tcp = TcpStream::connect(format!("{}:{}", host, port)).map_err(|e| format!("Connect error: {}", e))?;
            let mut sess = Session::new().map_err(|e: ssh2::Error| e.to_string())?;
            sess.set_tcp_stream(tcp);
            log_debug(&app_handle, "Starting handshake");
            sess.handshake().map_err(|e| format!("Handshake error: {}", e))?;

            match sess.auth_methods(&user) {
                Ok(methods) => log_debug(&app_handle, &format!("Server auth methods: {}", methods)),
                Err(e) => log_debug(&app_handle, &format!("Could not read server auth methods: {}", e)),
            }

            let mut authenticated = try_agent_auth(&sess, &user, &app_handle);

            if !authenticated {
                if let Some(key_path_str) = private_key.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
                    let path = resolve_private_key_path(key_path_str);
                    authenticated = try_pubkey_auth(&sess, &user, &path, passphrase, &app_handle, "configured key");
                }
            }

            if !authenticated {
                let configured_key = private_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(resolve_private_key_path);

                for candidate in default_key_candidates() {
                    if configured_key.as_ref().is_some_and(|configured| configured == &candidate) {
                        continue;
                    }

                    if try_pubkey_auth(&sess, &user, &candidate, passphrase, &app_handle, "default ~/.ssh key") {
                        authenticated = true;
                        break;
                    }
                }
            }

            if !authenticated {
                if let Some(pass) = auth_password.as_deref() {
                    log_debug(&app_handle, "Attempting password authentication.");
                    sess.userauth_password(&user, pass)
                        .map_err(|e| format!("{}:{}", PASSWORD_AUTH_FAILED_CODE, e))?;

                    if !sess.authenticated() {
                        return Err(format!("{}:Authentication failed", PASSWORD_AUTH_FAILED_CODE));
                    }
                    authenticated = true;
                    log_debug(&app_handle, "Password authentication succeeded.");
                } else {
                    log_debug(&app_handle, "Authentication requires a password prompt after agent/key attempts.");
                    return Err(PASSWORD_REQUIRED_CODE.to_string());
                }
            }

            if !authenticated || !sess.authenticated() {
                return Err("Authentication failed".to_string());
            }

            log_debug(&app_handle, &format!("Authenticated. Requesting PTY with terminal type {}.", requested_terminal));
            let mut channel = sess.channel_session().map_err(|e: ssh2::Error| e.to_string())?;
            channel.request_pty(&requested_terminal, None, Some((cols, rows, 0, 0))).map_err(|e| format!("PTY error: {}", e))?;
            match channel.setenv("TERM", &requested_terminal) {
                Ok(_) => log_debug(&app_handle, &format!("Exported TERM={} on SSH channel.", requested_terminal)),
                Err(e) => log_debug(&app_handle, &format!("SSH server rejected TERM export for {}: {}", requested_terminal, e)),
            }
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
