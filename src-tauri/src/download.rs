// Shared download/extract helpers used by tool installers (adb platform-tools, scrcpy).

use std::path::Path;
use std::time::Duration;

pub const USER_AGENT: &str = "ADBBar";

pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

fn status_error(status: reqwest::StatusCode, url: &str) -> String {
    if status.as_u16() == 403 || status.as_u16() == 429 {
        format!(
            "Request to {} was rate limited ({}). Wait a while and try again.",
            url, status
        )
    } else {
        format!("Request to {} failed with status {}", url, status)
    }
}

/// GET a text body (e.g. the GitHub releases API) with a total timeout and
/// an explicit HTTP status check so failures are reported by cause.
// Only referenced from Windows install code; keep it compiled everywhere so it
// stays type-checked and testable on the CI platform.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub async fn fetch_text(url: &str, timeout: Duration) -> Result<String, String> {
    let response = http_client()?
        .get(url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status, url));
    }
    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

/// Download a binary body, emitting progress messages like "Downloading... 42%"
/// whenever the whole-percent value changes. Connects with a timeout but no total
/// timeout, so large downloads on slow networks are not cut off mid-transfer.
pub async fn download_with_progress<F>(url: &str, emit: F) -> Result<Vec<u8>, String>
where
    F: Fn(String),
{
    let mut response = http_client()?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status, url));
    }
    let total = response.content_length();

    let mut data = Vec::new();
    let mut last_reported_percent = None;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Download interrupted: {}", e))?
    {
        data.extend_from_slice(&chunk);
        if let Some(percent) = progress_percent(data.len() as u64, total) {
            if last_reported_percent != Some(percent) {
                last_reported_percent = Some(percent);
                emit(format!("Downloading... {}%", percent));
            }
        }
    }
    if data.is_empty() {
        return Err(format!("Download from {} returned no data", url));
    }
    Ok(data)
}

fn progress_percent(downloaded: u64, total: Option<u64>) -> Option<u64> {
    let total = total?;
    if total == 0 {
        return None;
    }
    Some(downloaded * 100 / total)
}

/// Escape a value for use as a PowerShell single-quoted string literal.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Extract a zip file. Blocking; call from `spawn_blocking`.
#[cfg(target_os = "windows")]
pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let output = crate::adb::new_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "Expand-Archive -LiteralPath {} -DestinationPath {} -Force",
                ps_quote(&zip_path.to_string_lossy()),
                ps_quote(&dest_dir.to_string_lossy())
            ),
        ])
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to extract zip: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// Extract a zip file. Blocking; call from `spawn_blocking`.
#[cfg(not(target_os = "windows"))]
pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let output = std::process::Command::new("unzip")
        .arg("-o")
        .arg(zip_path)
        .arg("-d")
        .arg(dest_dir)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "unzip is not installed. Install it (e.g. sudo apt install unzip) and try again."
                    .to_string()
            } else {
                format!("Failed to run unzip: {}", e)
            }
        })?;
    if !output.status.success() {
        return Err(format!(
            "Failed to extract zip: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{progress_percent, ps_quote};

    #[test]
    fn ps_quote_escapes_single_quotes() {
        assert_eq!(ps_quote("plain"), "'plain'");
        assert_eq!(ps_quote("it's"), "'it''s'");
        assert_eq!(ps_quote("a'b'c"), "'a''b''c'");
    }

    #[test]
    fn progress_percent_handles_known_and_unknown_totals() {
        assert_eq!(progress_percent(0, Some(100)), Some(0));
        assert_eq!(progress_percent(50, Some(100)), Some(50));
        assert_eq!(progress_percent(120, Some(100)), Some(120));
        assert_eq!(progress_percent(10, Some(0)), None);
        assert_eq!(progress_percent(10, None), None);
    }
}
