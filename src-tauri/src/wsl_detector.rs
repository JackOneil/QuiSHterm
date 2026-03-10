use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::AppHandle;
use crate::settings_storage::log_debug;

#[derive(serde::Serialize, Clone)]
pub struct WslDistribution {
    pub name: String,
    pub is_default: bool,
}

#[tauri::command]
pub async fn get_wsl_distributions(app: AppHandle) -> Result<Vec<WslDistribution>, String> {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        log_debug(&app, "Executing cmd /C wsl.exe -l -q to fetch distributions");
        
        match Command::new("cmd")
            .args(["/C", "wsl.exe", "-l", "-q"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(output) => {
                let stdout = output.stdout;
                log_debug(&app, &format!("wsl.exe raw stdout bytes: {:?}", stdout));
                
                // WSL na Windows vrací UTF-16LE, zkusíme ho jednoduše přečíst a odstranit NUL byty
                let wsl_str = if stdout.len() % 2 == 0 && stdout.starts_with(&[0xff, 0xfe]) {
                    // Startuje s UTF-16 BOM
                    let u16_chars: Vec<u16> = stdout[2..]
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    String::from_utf16_lossy(&u16_chars)
                } else if stdout.len() % 2 == 0 {
                     let u16_chars: Vec<u16> = stdout
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    String::from_utf16_lossy(&u16_chars)
                } else {
                    String::from_utf8_lossy(&stdout).to_string()
                };

                // Fallback: If From UTF-16 gives weird chars, let's also try removing nulls outright
                // Sometimes it's just ASCII with \0 interleaved.
                let wsl_clean_str = wsl_str.replace('\0', "");
                log_debug(&app, &format!("wsl.exe parsed string: {:?}", wsl_clean_str));

                let distros: Vec<WslDistribution> = wsl_clean_str
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    // Sometimes wsl appends an asterisk to the default distro
                    .map(|line| line.trim_start_matches('*').trim().to_string())
                    .map(|name| WslDistribution {
                        name,
                        is_default: false,
                    })
                    .collect();

                log_debug(&app, &format!("wsl.exe detected parsed distros count: {}", distros.len()));
                Ok(distros)
            }
            Err(e) => {
                let err_msg = format!("WSL not found or failed: {}", e);
                log_debug(&app, &err_msg);
                Err(err_msg)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Return empty list on non-Windows
        Ok(vec![])
    }
}
