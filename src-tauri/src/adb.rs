use crate::store::{AdbDevice, StoreManager};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[allow(unused_mut)]
fn new_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

pub struct AdbService {
    adb_path: Mutex<String>,
}

impl AdbService {
    pub fn new(configured_path: Option<String>) -> Self {
        let path = configured_path
            .unwrap_or_else(|| Self::detect_adb_path().unwrap_or_else(|| "adb".to_string()));
        Self {
            adb_path: Mutex::new(path),
        }
    }

    pub fn detect_adb_path() -> Option<String> {
        // Check PATH environment
        if let Ok(path_var) = std::env::var("PATH") {
            let separator = if cfg!(windows) { ';' } else { ':' };
            for dir in path_var.split(separator) {
                let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };
                let candidate = PathBuf::from(dir).join(adb_name);
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }

        // Platform-specific locations
        #[cfg(target_os = "macos")]
        {
            let candidates = [
                "/opt/homebrew/bin/adb",
                &format!(
                    "{}/Library/Android/sdk/platform-tools/adb",
                    dirs::home_dir()?.to_string_lossy()
                ),
                "/usr/local/bin/adb",
            ];
            for candidate in &candidates {
                let p = PathBuf::from(candidate);
                if p.exists() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let candidate =
                    PathBuf::from(local_app_data).join("Android/Sdk/platform-tools/adb.exe");
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }

        None
    }

    pub async fn validate_adb_path(path: &str) -> Result<(), String> {
        let path = path.trim().to_string();
        if path.is_empty() {
            return Err("ADB path cannot be empty".to_string());
        }

        tokio::task::spawn_blocking(move || {
            let mut cmd = new_command(&path);
            cmd.arg("version");
            let output = cmd.output().map_err(|e| format!("Failed to run adb: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let message = if stderr.is_empty() { stdout } else { stderr };
                Err(format!("ADB validation failed: {}", message))
            }
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    pub async fn run(&self, args: &[&str], timeout_secs: u64) -> Result<String, String> {
        let adb_path = self.adb_path.lock().await.clone();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        tokio::task::spawn_blocking(move || {
            let mut cmd = new_command(&adb_path);
            cmd.args(&args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn adb: {}", e))?;

            let timeout = Duration::from_secs(timeout_secs);
            let start = std::time::Instant::now();

            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let mut stdout = String::new();
                        let mut stderr = String::new();
                        if let Some(mut out) = child.stdout.take() {
                            let _ = out.read_to_string(&mut stdout);
                        }
                        if let Some(mut err) = child.stderr.take() {
                            let _ = err.read_to_string(&mut stderr);
                        }
                        if status.success() {
                            return Ok(stdout.trim().to_string());
                        } else {
                            let err_msg = if stderr.trim().is_empty() {
                                stdout.trim().to_string()
                            } else {
                                stderr.trim().to_string()
                            };
                            return Err(format!("adb exited with status {}: {}", status, err_msg));
                        }
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            return Err(format!(
                                "adb command timed out after {} seconds",
                                timeout_secs
                            ));
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => return Err(format!("Failed to check adb status: {}", e)),
                }
            }
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    pub async fn connect(&self, address: &str) -> Result<String, String> {
        self.run(&["connect", address], 10).await
    }

    pub async fn disconnect(&self, address: &str) -> Result<String, String> {
        self.run(&["disconnect", address], 10).await
    }

    pub async fn get_devices(&self) -> Result<HashMap<String, String>, String> {
        let output = self.run(&["devices"], 5).await?;
        let mut devices = HashMap::new();

        for line in output.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
            if parts.len() == 2 {
                let address = parts[0].trim().to_string();
                let status = match parts[1].trim() {
                    "device" => "connected".to_string(),
                    "unauthorized" => "unauthorized".to_string(),
                    "offline" => "offline".to_string(),
                    other => other.to_string(),
                };
                devices.insert(address, status);
            }
        }

        Ok(devices)
    }

    pub async fn open_shell(&self, address: &str) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let adb_path = self.adb_path.lock().await.clone();
            let command = format!(
                "{} -s {} shell",
                shell_quote(&adb_path),
                shell_quote(address)
            );
            let script = format!(
                "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
                applescript_string_literal(&command)
            );
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .spawn()
                    .map_err(|e| format!("Failed to open Terminal: {}", e))?;
                Ok(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))??;
        }

        #[cfg(target_os = "windows")]
        {
            let adb_path = self.adb_path.lock().await.clone();
            let addr = address.to_string();
            tokio::task::spawn_blocking(move || {
                std::process::Command::new("cmd")
                    .args(["/C", "start", "", &adb_path, "-s", &addr, "shell"])
                    .spawn()
                    .map_err(|e| format!("Failed to open cmd: {}", e))?;
                Ok::<(), String>(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))??;
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = address;
            return Err("Opening shell is not supported on this platform".to_string());
        }

        Ok(())
    }

    pub async fn take_screenshot(&self, address: &str) -> Result<String, String> {
        let desktop = dirs::desktop_dir().ok_or("Cannot determine desktop directory")?;
        let timestamp = chrono_date_string();
        let filename = format!("screenshot_{}.png", timestamp);
        let filepath = desktop.join(&filename);

        let adb_path = self.adb_path.lock().await.clone();
        let addr = address.to_string();
        let fp = filepath.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Use adb exec-out and pipe to file directly
            let tmp_device = "/data/local/tmp/screenshot_adbbar.png";

            // Take screenshot on device
            let status = new_command(&adb_path)
                .args(["-s", &addr, "shell", "screencap", "-p", tmp_device])
                .output()
                .map_err(|e| format!("Failed to take screenshot: {}", e))?;

            if !status.status.success() {
                return Err(format!(
                    "screencap failed: {}",
                    String::from_utf8_lossy(&status.stderr)
                ));
            }

            // Pull to desktop
            let status = new_command(&adb_path)
                .args(["-s", &addr, "pull", tmp_device, &fp.to_string_lossy()])
                .output()
                .map_err(|e| format!("Failed to pull screenshot: {}", e))?;

            if !status.status.success() {
                return Err(format!(
                    "adb pull failed: {}",
                    String::from_utf8_lossy(&status.stderr)
                ));
            }

            // Clean up temp file on device
            let _ = new_command(&adb_path)
                .args(["-s", &addr, "shell", "rm", tmp_device])
                .output();

            Ok(fp.to_string_lossy().to_string())
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

        Ok(result)
    }

    pub async fn install_apk(&self, address: &str, apk_path: &str) -> Result<String, String> {
        self.run(&["-s", address, "install", "-r", apk_path], 120)
            .await
    }

    pub async fn refresh_statuses(
        adb: Arc<AdbService>,
        store: Arc<StoreManager>,
    ) -> Result<Vec<AdbDevice>, String> {
        let adb_devices = adb.get_devices().await?;
        store.update_statuses(&adb_devices).await?;
        let guard = store.store.lock().await;
        Ok(guard.devices.clone())
    }

    pub async fn set_adb_path(&self, path: String) {
        let mut guard = self.adb_path.lock().await;
        *guard = path;
    }

    pub async fn get_adb_path(&self) -> String {
        self.adb_path.lock().await.clone()
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn applescript_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn chrono_date_string() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
