use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Clone, Debug)]
pub struct ScrcpyStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
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

    pub async fn detect(&self) -> ScrcpyStatus {
        // Check PATH environment
        if let Ok(path_var) = std::env::var("PATH") {
            let separator = if cfg!(windows) { ';' } else { ':' };
            for dir in path_var.split(separator) {
                let scrcpy_name = if cfg!(windows) { "scrcpy.exe" } else { "scrcpy" };
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
                let candidate =
                    PathBuf::from(local_app_data).join("scrcpy").join("scrcpy.exe");
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

        ScrcpyStatus {
            installed: false,
            path: None,
            version: None,
        }
    }

    async fn get_version(&self, path: &str) -> Option<String> {
        let path = path.to_string();
        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new(&path)
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
                .args(&["install", "scrcpy"])
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
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/Genymobile/scrcpy/releases/latest")
            .header("User-Agent", "ADBBar")
            .send()
            .await
            .map_err(|e| format!("Failed to check latest release: {}", e))?;

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read release info: {}", e))?;
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

        let zip_response = client
            .get(download_url)
            .header("User-Agent", "ADBBar")
            .send()
            .await
            .map_err(|e| format!("Failed to download: {}", e))?;

        let zip_bytes = zip_response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read download: {}", e))?;

        let data_dir = dirs::data_dir().ok_or("Cannot determine app data directory")?;
        let install_dir = data_dir.join("adbbar").join("scrcpy");
        let _ = std::fs::create_dir_all(&install_dir);

        let zip_path = install_dir.join("scrcpy.zip");
        std::fs::write(&zip_path, &zip_bytes)
            .map_err(|e| format!("Failed to write zip: {}", e))?;

        emit("Extracting...".to_string());

        // Extract zip using PowerShell as fallback (avoids needing the zip crate)
        let extract_status = std::process::Command::new("powershell")
            .args(&[
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    zip_path.to_string_lossy(),
                    install_dir.to_string_lossy()
                ),
            ])
            .output()
            .map_err(|e| format!("Failed to extract zip: {}", e))?;

        if !extract_status.status.success() {
            return Err(format!(
                "Failed to extract zip: {}",
                String::from_utf8_lossy(&extract_status.stderr)
            ));
        }

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

    pub async fn launch(&self, address: &str) -> Result<(), String> {
        let scrcpy_path = {
            let guard = self.path.lock().await;
            guard.clone()
        };

        let scrcpy_path = scrcpy_path
            .ok_or("scrcpy is not installed. Please install it first.")?;

        let addr = address.to_string();
        tokio::task::spawn_blocking(move || {
            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("open")
                    .arg("-a")
                    .arg("Terminal")
                    .arg("--")
                    .arg(&scrcpy_path)
                    .arg("-s")
                    .arg(&addr)
                    .spawn()
                    .map_err(|e| format!("Failed to launch scrcpy: {}", e))?;
            }

            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("cmd")
                    .args(&["/C", "start", &scrcpy_path, "-s", &addr])
                    .spawn()
                    .map_err(|e| format!("Failed to launch scrcpy: {}", e))?;
            }

            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                std::process::Command::new(&scrcpy_path)
                    .arg("-s")
                    .arg(&addr)
                    .spawn()
                    .map_err(|e| format!("Failed to launch scrcpy: {}", e))?;
            }

            Ok(())
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }
}
