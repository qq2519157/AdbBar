use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
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

#[derive(Serialize, Clone, Debug)]
pub struct ScrcpyStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

/// Launch options read from the persistent store; all fields optional so the
/// scrcpy defaults apply when unset.
#[derive(serde::Deserialize, Clone, Copy, Debug, Default)]
pub struct ScrcpyLaunchOptions {
    pub bitrate_mbps: Option<u32>,
    pub turn_screen_off: bool,
    pub max_size: Option<u32>,
    pub stay_awake: bool,
}

pub struct ScrcpyService {
    pub path: Mutex<Option<String>>,
}

impl ScrcpyService {
    pub fn new() -> Self {
        Self {
            path: Mutex::new(None),
        }
    }

    pub async fn set_path(&self, path: String) {
        let mut guard = self.path.lock().await;
        *guard = Some(path);
    }

    pub async fn validate_path(path: &str) -> Result<(), String> {
        let path = path.trim().to_string();
        if path.is_empty() {
            return Err("scrcpy path cannot be empty".to_string());
        }
        if !PathBuf::from(&path).exists() {
            return Err(format!("File not found: {}", path));
        }
        tokio::task::spawn_blocking(move || {
            let output = new_command(&path)
                .arg("--version")
                .output()
                .map_err(|e| format!("Failed to run scrcpy: {}", e))?;
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(format!("Invalid scrcpy: {}", stderr))
            }
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    pub async fn detect(&self) -> ScrcpyStatus {
        // Check PATH environment
        if let Ok(path_var) = std::env::var("PATH") {
            let separator = if cfg!(windows) { ';' } else { ':' };
            for dir in path_var.split(separator) {
                let scrcpy_name = if cfg!(windows) {
                    "scrcpy.exe"
                } else {
                    "scrcpy"
                };
                let candidate = PathBuf::from(dir).join(scrcpy_name);
                if candidate.exists() {
                    let path_str = candidate.to_string_lossy().to_string();
                    let version = self.get_version(&path_str).await;
                    let mut guard = self.path.lock().await;
                    *guard = Some(path_str.clone());
                    return ScrcpyStatus {
                        installed: true,
                        path: Some(path_str),
                        version,
                    };
                }
            }
        }

        // Platform-specific locations
        #[cfg(target_os = "macos")]
        {
            let candidates = ["/opt/homebrew/bin/scrcpy", "/usr/local/bin/scrcpy"];
            for candidate in &candidates {
                let p = PathBuf::from(candidate);
                if p.exists() {
                    let path_str = p.to_string_lossy().to_string();
                    let version = self.get_version(&path_str).await;
                    let mut guard = self.path.lock().await;
                    *guard = Some(path_str.clone());
                    return ScrcpyStatus {
                        installed: true,
                        path: Some(path_str),
                        version,
                    };
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let candidate = PathBuf::from(local_app_data)
                    .join("scrcpy")
                    .join("scrcpy.exe");
                if candidate.exists() {
                    let path_str = candidate.to_string_lossy().to_string();
                    let version = self.get_version(&path_str).await;
                    let mut guard = self.path.lock().await;
                    *guard = Some(path_str.clone());
                    return ScrcpyStatus {
                        installed: true,
                        path: Some(path_str),
                        version,
                    };
                }
            }
        }

        let mut guard = self.path.lock().await;
        *guard = None;

        ScrcpyStatus {
            installed: false,
            path: None,
            version: None,
        }
    }

    async fn get_version(&self, path: &str) -> Option<String> {
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let output = new_command(&path)
                .arg("--version")
                .output()
                .ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().next().map(|s| s.trim().to_string())
        })
        .await
        .ok()
        .flatten()
    }

    pub async fn install<F>(&self, emit_event: F) -> Result<String, String>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        #[cfg(target_os = "macos")]
        {
            self.install_macos(emit_event).await
        }

        #[cfg(target_os = "windows")]
        {
            self.install_windows(emit_event).await
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = emit_event;
            Err("Automatic installation is not supported on this platform. Please install scrcpy manually.".to_string())
        }
    }

    #[cfg(target_os = "macos")]
    async fn install_macos<F>(&self, emit_event: F) -> Result<String, String>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let emit = Arc::new(emit_event);
        emit("Installing scrcpy via Homebrew...".to_string());

        let emit_clone = emit.clone();
        let result = tokio::task::spawn_blocking(move || {
            let child = std::process::Command::new("brew")
                .args(["install", "scrcpy"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to run brew: {}", e))?;

            let output = child
                .wait_with_output()
                .map_err(|e| format!("Failed to wait for brew: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            emit_clone(format!("{}\n{}", stdout, stderr));

            if output.status.success() {
                Ok("scrcpy installed successfully via Homebrew".to_string())
            } else {
                Err(format!("brew install failed: {}", stderr))
            }
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

        // Re-detect after install
        let status = self.detect().await;
        if status.installed {
            Ok(result)
        } else {
            Err("Installation completed but scrcpy was not found".to_string())
        }
    }

    #[cfg(target_os = "windows")]
    async fn install_windows<F>(&self, emit_event: F) -> Result<String, String>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let emit = Arc::new(emit_event);
        emit("Downloading scrcpy for Windows...".to_string());

        // Get latest release URL
        let body = crate::download::fetch_text(
            "https://api.github.com/repos/Genymobile/scrcpy/releases/latest",
            std::time::Duration::from_secs(30),
        )
        .await?;
        let release: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse release info: {}", e))?;

        let assets = release["assets"]
            .as_array()
            .ok_or("No assets found in release")?;

        let download_url = assets
            .iter()
            .find(|a| {
                a["name"]
                    .as_str()
                    .map(|n| n.starts_with("scrcpy-win64-") && n.ends_with(".zip"))
                    .unwrap_or(false)
            })
            .and_then(|a| a["browser_download_url"].as_str())
            .ok_or("Could not find Windows zip download")?;

        emit(format!("Downloading from {}...", download_url));

        let zip_bytes = crate::download::download_with_progress(download_url, {
            let emit = emit.clone();
            move |msg: String| emit(msg)
        })
        .await?;

        let data_dir = dirs::data_dir().ok_or("Cannot determine app data directory")?;
        let install_dir = data_dir.join("adbbar").join("scrcpy");
        let _ = std::fs::create_dir_all(&install_dir);

        let zip_path = install_dir.join("scrcpy.zip");
        std::fs::write(&zip_path, &zip_bytes).map_err(|e| format!("Failed to write zip: {}", e))?;

        emit("Extracting...".to_string());

        // Extract zip using PowerShell (avoids needing the zip crate); runs hidden
        // (CREATE_NO_WINDOW) with PS-quoted paths.
        let zip = zip_path.clone();
        let dir = install_dir.clone();
        tokio::task::spawn_blocking(move || crate::download::extract_zip(&zip, &dir))
            .await
            .map_err(|e| format!("Task join error: {}", e))??;

        // Clean up zip
        let _ = std::fs::remove_file(&zip_path);

        // Find scrcpy.exe in extracted files
        let scrcpy_exe = Self::find_file_recursive(&install_dir, "scrcpy.exe")
            .ok_or("scrcpy.exe not found in archive")?;

        let mut guard = self.path.lock().await;
        *guard = Some(scrcpy_exe.to_string_lossy().to_string());

        emit("scrcpy installed successfully".to_string());

        Ok("scrcpy installed successfully".to_string())
    }

    #[cfg(target_os = "windows")]
    fn find_file_recursive(dir: &PathBuf, name: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = Self::find_file_recursive(&path, name) {
                    return Some(found);
                }
            } else if path.file_name()?.to_str()? == name {
                return Some(path);
            }
        }
        None
    }

    pub async fn launch(
        &self,
        address: &str,
        options: ScrcpyLaunchOptions,
    ) -> Result<(), String> {
        let scrcpy_path = {
            let guard = self.path.lock().await;
            guard.clone()
        };

        let scrcpy_path = scrcpy_path.ok_or("scrcpy is not installed. Please install it first.")?;

        let addr = address.to_string();
        tokio::task::spawn_blocking(move || {
            let mut cmd = new_command(&scrcpy_path);
            cmd.arg("-s").arg(&addr);
            if let Some(mbps) = options.bitrate_mbps {
                cmd.arg("--video-bit-rate").arg(format!("{}M", mbps));
            }
            if let Some(size) = options.max_size {
                // Limit both dimensions to this value (long side).
                cmd.arg("--max-size").arg(size.to_string());
            }
            if options.turn_screen_off {
                // Keep mirroring with the device screen off.
                cmd.arg("-S");
            }
            if options.stay_awake {
                cmd.arg("--stay-awake");
            }
            cmd.spawn()
                .map_err(|e| format!("Failed to launch scrcpy: {}", e))?;

            Ok(())
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }
}
