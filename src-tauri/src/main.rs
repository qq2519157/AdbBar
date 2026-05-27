#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

fn main() {
    adbbar_tauri::run_builder(tray_setup);
}

fn tray_setup(app: &tauri::App) {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>).unwrap();
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
    let menu = Menu::with_items(app, &[&show, &quit]).unwrap();

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("ADB Bar")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
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
                        position_window_near_tray(&window, tray);
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)
        .expect("Failed to create tray icon");
}

fn position_window_near_tray(window: &tauri::WebviewWindow, tray: &tauri::tray::TrayIcon) {
    let tray_rect = match tray.rect() {
        Ok(Some(r)) => r,
        _ => return,
    };

    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let logical_size = window
        .inner_size()
        .map(|s| s.to_logical::<f64>(scale_factor))
        .unwrap_or(tauri::LogicalSize::new(320.0, 480.0));

    let win_w = logical_size.width as i32;
    let (tray_x, tray_y) = match tray_rect.position {
        tauri::Position::Logical(p) => (p.x as i32, p.y as i32),
        tauri::Position::Physical(p) => (p.x, p.y),
    };
    let tray_h = match tray_rect.size {
        tauri::Size::Logical(s) => s.height as i32,
        tauri::Size::Physical(s) => s.height as i32,
    };

    #[cfg(target_os = "macos")]
    let y = (tray_y + tray_h + 4) as f64;

    #[cfg(not(target_os = "macos"))]
    let y = (tray_y - logical_size.height as i32 - 4) as f64;

    let x = (tray_x - win_w / 2 + 16) as f64;
    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
}
