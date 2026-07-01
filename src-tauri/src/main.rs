#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use adbbar_tauri::{adb::AdbService, AppState};
use tauri::{
    image::Image,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder},
    Emitter, Manager,
};

#[cfg(target_os = "macos")]
fn notify(title: &str, message: &str) {
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message.replace('\\', "\\\\").replace('"', "\\\""),
        title.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output();
}

#[cfg(target_os = "windows")]
fn notify(title: &str, message: &str) {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms;\
         $n = New-Object System.Windows.Forms.NotifyIcon;\
         $n.Icon = [System.Drawing.SystemIcons]::Information;\
         $n.BalloonTipTitle = '{}';\
         $n.BalloonTipText = '{}';\
         $n.Visible = $true;\
         $n.ShowBalloonTip(3000);\
         Start-Sleep -Seconds 3;\
         $n.Dispose()",
        title.replace('\'', "''"),
        message.replace('\'', "''")
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn();
}

#[cfg(target_os = "linux")]
fn notify(title: &str, message: &str) {
    let _ = std::process::Command::new("notify-send")
        .arg(title)
        .arg(message)
        .spawn();
}

fn main() {
    adbbar_tauri::run_builder(tray_setup);
}

fn tray_setup(app: &tauri::App) {
    let state = app.state::<AppState>();
    let devices = tauri::async_runtime::block_on(state.store.store.lock()).devices.clone();
    let menu = adbbar_tauri::build_tray_menu(app, &devices);
    let tray_icon = tray_icon_image();

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon)
        .icon_as_template(true)
        .tooltip("ADB Bar")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|tray, event| {
            let app = tray.app_handle();
            let id = event.id.as_ref().to_string();
            match id.as_str() {
                "quit" => {
                    app.exit(0);
                }
                "restart-adb" => {
                    let adb = app.state::<AppState>().adb.clone();
                    let ah = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let result = async {
                            adb.run(&["kill-server"], 5).await?;
                            adb.run(&["start-server"], 10).await
                        }
                        .await;
                        match result {
                            Ok(_) => {
                                notify("ADB Bar", &adbbar_tauri::locale::notify_text("restart_success"));
                                let _ = ah.emit("adb-restarted", ());
                            }
                            Err(e) => {
                                notify("ADB Bar", &adbbar_tauri::locale::notify_text("restart_failed"));
                                eprintln!("restart_adb error: {e}");
                            }
                        }
                    });
                }
                "enable-tcpip" => {
                    let adb = app.state::<AppState>().adb.clone();
                    let ah = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let result = adb.run(&["tcpip", "5555"], 10).await;
                        match result {
                            Ok(_) => {
                                notify("ADB Bar", &adbbar_tauri::locale::notify_text("tcpip_success"));
                                let _ = ah.emit("tcpip-enabled", ());
                            }
                            Err(e) => {
                                notify("ADB Bar", &adbbar_tauri::locale::notify_text("tcpip_failed"));
                                eprintln!("enable_tcpip error: {e}");
                            }
                        }
                    });
                }
                s if s.starts_with("connect:") => {
                    let addr = s[8..].to_string();
                    let state = app.state::<AppState>();
                    let adb = state.adb.clone();
                    let store = state.store.clone();
                    let ah = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let result = adb.connect(&addr).await;
                        match result {
                            Ok(_) => {
                                notify("ADB Bar", &adbbar_tauri::locale::notify_text("connect_success"));
                            }
                            Err(e) => {
                                notify("ADB Bar", &adbbar_tauri::locale::notify_text("connect_failed"));
                                eprintln!("connect error: {e}");
                            }
                        }
                        let _ = AdbService::refresh_statuses(adb.clone(), store.clone()).await;
                        let guard = store.store.lock().await;
                        let devices = guard.devices.clone();
                        drop(guard);
                        adbbar_tauri::rebuild_tray_menu(&ah, devices.clone());
                        let _ = ah.emit("devices-updated", &devices);
                    });
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        if let Ok(Some(rect)) = tray.rect() {
                            use tauri::{LogicalPosition, Position, Size};
                            let scale = window.scale_factor().unwrap_or(1.0);
                            let (tray_x, tray_y) = match rect.position {
                                Position::Physical(p) => (p.x as f64 / scale, p.y as f64 / scale),
                                Position::Logical(p) => (p.x, p.y),
                            };
                            let tray_height = match rect.size {
                                Size::Physical(s) => s.height as f64 / scale,
                                Size::Logical(s) => s.height,
                            };
                            let _win_w = window
                                .inner_size()
                                .unwrap_or_else(|_| tauri::PhysicalSize::new(320, 480))
                                .width as f64
                                / scale;
                            let x = tray_x;
                            #[cfg(target_os = "windows")]
                            let win_h = window.inner_size().unwrap_or_else(|_| tauri::PhysicalSize::new(320, 480)).height as f64 / scale;
                            #[cfg(target_os = "windows")]
                            let y = tray_y - win_h;
                            #[cfg(not(target_os = "windows"))]
                            let y = tray_y + tray_height;
                            let _ = window.set_position(LogicalPosition::new(x, y));
                        }
                        let _ = window.show();
                        let _ = window.set_focus();
                        adbbar_tauri::mark_just_shown();
                        let _ = app.emit("window-shown", ());
                    }
                }
            }
        })
        .build(app)
        .expect("Failed to create tray icon");
}

fn tray_icon_image() -> Image<'static> {
    let rgba = include_bytes!("../icons/tray-adb.rgba");
    #[cfg(target_os = "windows")]
    {
        let mut data = rgba.to_vec();
        for pixel in data.chunks_exact_mut(4) {
            pixel[0] = 255 - pixel[0];
            pixel[1] = 255 - pixel[1];
            pixel[2] = 255 - pixel[2];
        }
        Image::new_owned(data, 22, 22)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Image::new(rgba, 22, 22).to_owned()
    }
}
