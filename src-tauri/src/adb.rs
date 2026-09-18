use crate::store::{AdbDevice, StoreManager};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[allow(unused_mut)]
pub(crate) fn new_command(program: &str) -> std::process::Command {
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

        // SDK location environment variables (Android Studio, CI setups)
        for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
            if let Ok(sdk_root) = std::env::var(var) {
                let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };
                let candidate = PathBuf::from(sdk_root).join("platform-tools").join(adb_name);
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
            if let Some(home) = dirs::home_dir() {
                let candidate = home.join("scoop").join("shims").join("adb.exe");
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
            let candidate = PathBuf::from("C:\\ProgramData\\chocolatey\\bin\\adb.exe");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
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

            // Drain stdout/stderr on dedicated threads while the child runs: reading only
            // after exit deadlocks once a pipe fills up (~64KB of output).
            let stdout_handle = spawn_output_reader(child.stdout.take());
            let stderr_handle = spawn_output_reader(child.stderr.take());

            let timeout = Duration::from_secs(timeout_secs);
            let start = std::time::Instant::now();

            let status = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            // Reap the killed child so it does not linger as a zombie.
                            let _ = child.wait();
                            let _ = stdout_handle.join();
                            let _ = stderr_handle.join();
                            return Err(format!(
                                "adb command timed out after {} seconds",
                                timeout_secs
                            ));
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => return Err(format!("Failed to check adb status: {}", e)),
                }
            };

            let stdout = stdout_handle.join().unwrap_or_default();
            let stderr = stderr_handle.join().unwrap_or_default();

            if status.success() {
                Ok(stdout.trim().to_string())
            } else {
                let err_msg = if stderr.trim().is_empty() {
                    stdout.trim().to_string()
                } else {
                    stderr.trim().to_string()
                };
                Err(format!("adb exited with status {}: {}", status, err_msg))
            }
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }

    pub async fn connect(&self, address: &str) -> Result<String, String> {
        let output = self.run(&["connect", address], 10).await?;
        // `adb connect` exits with status 0 even when the connection fails,
        // so the output text is the only reliable success signal.
        if connect_succeeded(&output) {
            Ok(output)
        } else if output.trim().is_empty() {
            Err(format!("adb connect produced no output for {}", address))
        } else {
            Err(output.trim().to_string())
        }
    }

    pub async fn disconnect(&self, address: &str) -> Result<String, String> {
        self.run(&["disconnect", address], 10).await
    }

    /// Pair with a device using the code shown in the phone's
    /// "Wireless debugging → Pair device with pairing code" screen.
    /// The pairing address/port is the one shown on that screen and is
    /// different from the later connect port.
    pub async fn pair(&self, address: &str, code: &str) -> Result<String, String> {
        let output = self.run(&["pair", address, code], 20).await?;
        if pair_succeeded(&output) {
            Ok(output)
        } else if output.trim().is_empty() {
            Err(format!("adb pair produced no output for {}", address))
        } else {
            Err(output.trim().to_string())
        }
    }

    pub async fn get_devices(&self) -> Result<HashMap<String, String>, String> {
        let output = self.run(&["devices"], 5).await?;
        Ok(parse_devices_output(&output))
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

    fn platform_tools_url() -> &'static str {
        if cfg!(target_os = "windows") {
            "https://dl.google.com/android/repository/platform-tools-latest-windows.zip"
        } else if cfg!(target_os = "macos") {
            "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip"
        } else {
            "https://dl.google.com/android/repository/platform-tools-latest-linux.zip"
        }
    }

    /// Download the official Android platform-tools, extract it into the app data
    /// directory and switch this service over to the extracted adb binary.
    /// Persisting the path is left to the command layer (mirrors set_adb_path).
    pub async fn install<F>(&self, emit: F) -> Result<String, String>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let emit = Arc::new(emit);

        emit("Downloading Android platform-tools...".to_string());
        let bytes = crate::download::download_with_progress(Self::platform_tools_url(), {
            let emit = emit.clone();
            move |msg: String| emit(msg)
        })
        .await?;

        let data_dir = dirs::data_dir()
            .ok_or("Cannot determine app data directory")?
            .join("adbbar");
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create install directory: {}", e))?;

        let zip_path = data_dir.join("platform-tools.zip");
        std::fs::write(&zip_path, &bytes)
            .map_err(|e| format!("Failed to write zip: {}", e))?;

        emit("Extracting...".to_string());
        let zip = zip_path.clone();
        let dir = data_dir.clone();
        tokio::task::spawn_blocking(move || crate::download::extract_zip(&zip, &dir))
            .await
            .map_err(|e| format!("Task join error: {}", e))??;
        let _ = std::fs::remove_file(&zip_path);

        let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };
        let adb_bin = data_dir.join("platform-tools").join(adb_name);
        if !adb_bin.exists() {
            return Err(format!(
                "adb binary not found in extracted archive: {}",
                adb_bin.to_string_lossy()
            ));
        }
        // Ensure the binary is executable even if the extractor dropped the mode bits.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&adb_bin, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to make adb executable: {}", e))?;
        }
        let adb_bin = adb_bin.to_string_lossy().to_string();

        emit("Validating adb...".to_string());
        Self::validate_adb_path(&adb_bin).await?;

        self.set_adb_path(adb_bin.clone()).await;
        emit("ADB installed successfully".to_string());
        Ok(adb_bin)
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

/// `adb connect` always exits 0, even for failures like
/// "failed to connect to 'host:port': Connection refused". Success is declared only by
/// a result line "connected to <addr>" or "already connected to <addr>"; lines starting
/// with '*' (daemon startup notices on older adb builds) are not result lines.
fn connect_succeeded(output: &str) -> bool {
    let result_line = output
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty() && !line.starts_with('*'));
    matches!(result_line, Some(line)
        if line.starts_with("connected to") || line.starts_with("already connected"))
}

/// `adb pair` announces success by printing a "Successfully paired to <addr>" line
/// (possibly after daemon notices); failures print "Failed to pair ..." / "error: ...".
/// Exit codes alone are not trusted, mirroring `adb connect`.
fn pair_succeeded(output: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim().to_lowercase().starts_with("successfully paired"))
}

/// Parse `adb shell getprop` output lines like `[ro.product.model]: [Pixel 8]`
/// into a key -> value map (sorted, since keys come out in arbitrary order).
pub(crate) fn parse_getprop_output(output: &str) -> std::collections::BTreeMap<String, String> {
    let mut props = std::collections::BTreeMap::new();
    for line in output.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once("]: [") else {
            continue;
        };
        let key = key.trim_start_matches('[').trim().to_string();
        if key.is_empty() {
            continue;
        }
        let value = value.strip_suffix(']').unwrap_or(value).to_string();
        props.insert(key, value);
    }
    props
}

#[derive(Serialize, Clone, Debug)]
pub struct MdnsService {
    pub name: String,
    /// "pairing" for _adb-tls-pairing services, "connect" for everything else.
    pub kind: String,
    /// Raw mDNS service type, e.g. "_adb._tcp" or "_adb-tls-pairing._tcp".
    pub service_type: String,
    /// ip:port advertised by the service.
    pub address: String,
}

/// Parse `adb mdns services` output into discovered services. Seen formats:
/// - openscreen backend (adb >= 33): `<name>\t<_adb...>\t<ip:port>` per line
/// - older bonjour backend: `<name>._adb-tls-pairing._tcp.\t<host>\t<ip:port>`
///
/// Header/preamble lines and lines without a parseable ip:port are skipped.
pub(crate) fn parse_mdns_services(output: &str) -> Vec<MdnsService> {
    let mut services = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if lower.starts_with("list of discovered")
            || lower.starts_with("mdns discovery")
            || lower.starts_with("adb mdns backend")
            || lower.starts_with("mdns daemon")
        {
            continue;
        }

        let mut name: Option<String> = None;
        let mut service_type: Option<String> = None;
        let mut address: Option<String> = None;

        for token in line.split_whitespace() {
            if token.parse::<std::net::SocketAddr>().is_ok() {
                address = Some(token.to_string());
            } else if let Some(rest) = token.strip_prefix("_adb") {
                service_type = Some(format!("_adb{}", rest));
            } else if token.contains("._adb") {
                if let Some((n, t)) = token.split_once("._adb") {
                    name = Some(n.to_string());
                    service_type = Some(format!("_adb{}", t));
                }
            } else if name.is_none() {
                name = Some(token.to_string());
            }
        }

        let (Some(name), Some(service_type), Some(address)) = (name, service_type, address) else {
            continue;
        };
        let kind = if service_type.contains("pairing") {
            "pairing"
        } else {
            "connect"
        };
        services.push(MdnsService {
            name,
            kind: kind.to_string(),
            service_type,
            address,
        });
    }
    services
}

fn spawn_output_reader<R: Read + Send + 'static>(
    pipe: Option<R>,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).to_string()
    })
}

/// Parse `adb devices` stdout into an address -> status map. A line only counts as a
/// device entry when it carries a status field; the "List of devices attached" header
/// and '*' daemon notices (printed to stdout by some adb builds) are skipped instead
/// of relying on the header always being the first line.
fn parse_devices_output(output: &str) -> HashMap<String, String> {
    let mut devices = HashMap::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') || line == "List of devices attached" {
            continue;
        }
        let Some((address, status)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let status = match status.trim() {
            "device" => "connected".to_string(),
            "unauthorized" => "unauthorized".to_string(),
            "offline" => "offline".to_string(),
            other => other.to_string(),
        };
        devices.insert(address.trim().to_string(), status);
    }
    devices
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

#[cfg(test)]
mod tests {
    use super::{connect_succeeded, parse_devices_output};

    #[test]
    fn parse_devices_output_maps_statuses() {
        let output = "List of devices attached\n\
                      192.168.1.5:5555\tdevice\n\
                      192.168.1.6:5555\toffline\n\
                      emulator-5554\tunauthorized\n";
        let devices = parse_devices_output(output);
        assert_eq!(devices.get("192.168.1.5:5555").unwrap(), "connected");
        assert_eq!(devices.get("192.168.1.6:5555").unwrap(), "offline");
        assert_eq!(devices.get("emulator-5554").unwrap(), "unauthorized");
        assert_eq!(devices.len(), 3);
    }

    #[test]
    fn parse_devices_output_skips_daemon_notices_and_header() {
        let output = "* daemon not running; starting now at tcp:5037\n\
                      * daemon started successfully\n\
                      List of devices attached\n\
                      192.168.1.5:5555\tdevice\n";
        let devices = parse_devices_output(output);
        assert_eq!(devices.len(), 1);
        assert!(devices.contains_key("192.168.1.5:5555"));
    }

    #[test]
    fn parse_devices_output_handles_empty_output() {
        assert!(parse_devices_output("").is_empty());
        assert!(parse_devices_output("List of devices attached\r\n\r\n").is_empty());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn platform_tools_url_matches_platform() {
        assert!(super::AdbService::platform_tools_url().contains("darwin"));
    }

    #[test]
    fn pair_succeeded_matches_success_lines() {
        assert!(super::pair_succeeded("Successfully paired to 192.168.1.5:37123"));
        assert!(super::pair_succeeded(
            "Successfully paired to 192.168.1.5:37123 [guid=adb-xyz]"
        ));
        // Daemon notices may precede the result line; match case-insensitively.
        assert!(super::pair_succeeded(
            "* daemon not running; starting now at tcp:5037\n\
             * daemon started successfully\n\
             successfully paired to 192.168.1.5:37123"
        ));
    }

    #[test]
    fn pair_succeeded_rejects_failure_outputs() {
        assert!(!super::pair_succeeded(
            "Failed to pair to 192.168.1.5:37123: wrong pairing code"
        ));
        assert!(!super::pair_succeeded("error: closed"));
        assert!(!super::pair_succeeded(""));
        // "daemon started successfully" alone is not a pairing result.
        assert!(!super::pair_succeeded(
            "* daemon not running; starting now at tcp:5037\n* daemon started successfully"
        ));
    }

    #[test]
    fn parse_getprop_output_extracts_key_values() {
        let output = "[ro.product.brand]: [google]\n\
                      [ro.product.model]: [Pixel 8]\n\
                      [ro.build.version.release]: [14]\n\
                      [ro.serialno]: []\n";
        let props = super::parse_getprop_output(output);
        assert_eq!(props.get("ro.product.brand").unwrap(), "google");
        assert_eq!(props.get("ro.product.model").unwrap(), "Pixel 8");
        assert_eq!(props.get("ro.build.version.release").unwrap(), "14");
        assert_eq!(props.get("ro.serialno").unwrap(), "");
        assert_eq!(props.len(), 4);
    }

    #[test]
    fn parse_getprop_output_skips_malformed_lines() {
        let output = "* daemon started successfully\n\
                      some stray line\n\
                      [not-a-pair line\n\
                      [ro.product.model]: [Pixel 8]\n";
        let props = super::parse_getprop_output(output);
        assert_eq!(props.len(), 1);
        assert!(props.contains_key("ro.product.model"));
    }

    #[test]
    fn parse_getprop_output_keeps_separator_inside_value() {
        // A value that itself contains "]: [" must not be cut in half.
        let props = super::parse_getprop_output("[weird.prop]: [a]: [b]");
        assert_eq!(props.get("weird.prop").unwrap(), "a]: [b");
    }

    #[test]
    fn parse_devices_output_passes_through_unknown_status() {
        let devices =
            super::parse_devices_output("List of devices attached\nXZY123\tsideload\n");
        assert_eq!(devices.get("XZY123").unwrap(), "sideload");
    }

    #[test]
    fn parse_mdns_services_openscreen_format() {
        // Exact shape observed from adb 36.0.0 (openscreen backend).
        let output = "List of discovered mdns services\n\
                      adb-unidentified\t_adb._tcp\t192.168.31.199:5555\n";
        let services = super::parse_mdns_services(output);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "adb-unidentified");
        assert_eq!(services[0].service_type, "_adb._tcp");
        assert_eq!(services[0].address, "192.168.31.199:5555");
        assert_eq!(services[0].kind, "connect");
    }

    #[test]
    fn parse_mdns_services_dotted_and_preambles() {
        let output = "adb MDNS backend [mDNS]\n\
                      mDNS discovery accelerate enabled.\n\
                      List of discovered MDNS services:\n\
                      adb-XYZ123._adb-tls-pairing._tcp.\tlocal._ipv4\t192.168.1.5:37123\n\
                      adb-XYZ123._adb-tls-connect._tcp.\tlocal._ipv4\t192.168.1.5:43567\n\
                      unrelated line without address\n";
        let services = super::parse_mdns_services(output);
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "adb-XYZ123");
        assert_eq!(services[0].service_type, "_adb-tls-pairing._tcp.");
        assert_eq!(services[0].kind, "pairing");
        assert_eq!(services[0].address, "192.168.1.5:37123");
        assert_eq!(services[1].kind, "connect");
        assert_eq!(services[1].address, "192.168.1.5:43567");
    }

    #[test]
    fn connect_succeeded_accepts_success_outputs() {
        assert!(connect_succeeded("connected to 192.168.1.5:5555"));
        assert!(connect_succeeded("already connected to 192.168.1.5:5555"));
        assert!(connect_succeeded("\r\nconnected to 192.168.1.5:5555\r\n"));
        // Older adb builds print daemon startup notices to stdout before the result.
        assert!(connect_succeeded(
            "* daemon not running; starting now at tcp:5037\n\
             * daemon started successfully\n\
             connected to 192.168.1.5:5555"
        ));
    }

    #[test]
    fn connect_succeeded_rejects_failure_outputs() {
        assert!(!connect_succeeded(
            "failed to connect to '192.168.1.5:5555': Connection refused"
        ));
        assert!(!connect_succeeded("cannot connect to 192.168.1.5:5555: Operation timed out"));
        assert!(!connect_succeeded("failed to authenticate to '192.168.1.5:5555'"));
        assert!(!connect_succeeded(""));
        assert!(!connect_succeeded(
            "* daemon not running; starting now at tcp:5037\n\
             * daemon started successfully"
        ));
    }
}

#[cfg(all(test, unix))]
mod run_tests {
    use super::AdbService;

    #[tokio::test]
    async fn run_captures_stdout() {
        let svc = AdbService::new(Some("/bin/echo".to_string()));
        let out = svc.run(&["hello"], 5).await.unwrap();
        assert_eq!(out, "hello");
    }

    #[tokio::test]
    async fn run_times_out_and_kills_child() {
        let svc = AdbService::new(Some("/bin/sleep".to_string()));
        let start = std::time::Instant::now();
        let err = svc.run(&["30"], 1).await.unwrap_err();
        assert!(err.contains("timed out"), "unexpected error: {}", err);
        assert!(
            start.elapsed().as_secs() < 5,
            "timeout took too long: {:?}",
            start.elapsed()
        );
    }
}

#[cfg(test)]
mod install_tests {
    use super::AdbService;

    // Real network + filesystem integration test: downloads platform-tools from
    // dl.google.com and extracts it into the app data directory. Run explicitly:
    //   cargo test --lib -- --ignored
    #[tokio::test]
    #[ignore = "downloads ~10MB and writes to the app data directory"]
    async fn install_downloads_and_extracts_real_adb() {
        let svc = AdbService::new(None);
        let path = svc
            .install(|_| {})
            .await
            .expect("adb install should succeed end to end");
        assert!(
            std::path::PathBuf::from(&path).exists(),
            "adb not found at {}",
            path
        );
    }
}

/// Real-device E2E tests through the actual service code paths. Never run in CI;
/// opt in with a device address, e.g.:
///   ADB_E2E_TARGET=192.168.31.199:5555 cargo test --lib -- --ignored
#[cfg(test)]
mod device_e2e_tests {
    use super::{parse_devices_output, parse_getprop_output, AdbService};

    fn target() -> Option<String> {
        std::env::var("ADB_E2E_TARGET").ok().filter(|s| !s.is_empty())
    }

    #[tokio::test]
    #[ignore = "requires a real device; set ADB_E2E_TARGET=ip:port"]
    async fn real_device_connect_devices_getprop_roundtrip() {
        let Some(address) = target() else {
            panic!("set ADB_E2E_TARGET=ip:port to run this test");
        };
        let svc = AdbService::new(None);

        // connect → parse live `adb devices` output through the real parser
        svc.connect(&address).await.expect("connect should succeed");
        let devices = svc.get_devices().await.expect("get_devices should succeed");
        let status = devices.get(&address).expect("device should be listed");
        assert_eq!(status, "connected", "unexpected status: {}", status);

        // getprop through the real service + parser
        let output = svc
            .run(&["-s", &address, "shell", "getprop"], 10)
            .await
            .expect("getprop should succeed");
        let props = parse_getprop_output(&output);
        assert!(
            props.contains_key("ro.build.version.release"),
            "missing ro.build.version.release in {} props",
            props.len()
        );
        assert!(parse_devices_output("").is_empty()); // sanity on the parser itself

        svc.disconnect(&address).await.expect("disconnect should succeed");
    }
}
