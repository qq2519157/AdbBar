use std::sync::atomic::{AtomicU8, Ordering};

// 0 = en, 1 = zh
static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(0);

pub fn init_locale(preference: Option<&str>) {
    let locale = preference.unwrap_or_else(|| detect_system_locale());
    set_current(locale);
}

fn set_current(locale: &str) {
    match locale {
        "zh" => CURRENT_LOCALE.store(1, Ordering::SeqCst),
        _ => CURRENT_LOCALE.store(0, Ordering::SeqCst),
    }
}

pub fn current_locale() -> &'static str {
    if CURRENT_LOCALE.load(Ordering::SeqCst) == 1 {
        "zh"
    } else {
        "en"
    }
}

pub fn set_locale(locale: &str) {
    set_current(locale);
}

fn detect_system_locale() -> &'static str {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_else(|_| "en_US.UTF-8".to_string());
    if lang.to_lowercase().starts_with("zh") {
        "zh"
    } else {
        "en"
    }
}

pub fn tray_text(key: &str) -> String {
    if current_locale() == "zh" {
        match key {
            "quick_connect" => "快速连接".to_string(),
            "no_devices" => "(无设备)".to_string(),
            "restart_adb" => "重启 ADB 服务".to_string(),
            "enable_tcpip" => "启用 TCP/IP (5555)".to_string(),
            "quit" => "退出".to_string(),
            _ => key.to_string(),
        }
    } else {
        match key {
            "quick_connect" => "Quick Connect".to_string(),
            "no_devices" => "(No devices)".to_string(),
            "restart_adb" => "Restart ADB Server".to_string(),
            "enable_tcpip" => "Enable TCP/IP (5555)".to_string(),
            "quit" => "Quit".to_string(),
            _ => key.to_string(),
        }
    }
}

pub fn notify_text(key: &str) -> String {
    if current_locale() == "zh" {
        match key {
            "restart_success" => "ADB 服务已重启".to_string(),
            "restart_failed" => "重启 ADB 失败".to_string(),
            "tcpip_success" => "TCP/IP 模式已启用".to_string(),
            "tcpip_failed" => "启用 TCP/IP 失败".to_string(),
            "connect_success" => "连接成功".to_string(),
            "connect_failed" => "连接失败".to_string(),
            _ => key.to_string(),
        }
    } else {
        match key {
            "restart_success" => "ADB server restarted".to_string(),
            "restart_failed" => "Failed to restart ADB".to_string(),
            "tcpip_success" => "TCP/IP mode enabled".to_string(),
            "tcpip_failed" => "Failed to enable TCP/IP".to_string(),
            "connect_success" => "Connected".to_string(),
            "connect_failed" => "Connection failed".to_string(),
            _ => key.to_string(),
        }
    }
}
