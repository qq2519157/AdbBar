#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder},
    Manager,
};

fn main() {
    adbbar_tauri::run_builder(tray_setup);
}

fn tray_setup(app: &tauri::App) {
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
    let menu = Menu::with_items(app, &[&quit]).unwrap();

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("ADB Bar")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                app.exit(0);
            }
            _ => {}
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
                            let tray_pos = match rect.position {
                                Position::Physical(p) => {
                                    let scale = window.scale_factor().unwrap_or(1.0);
                                    (p.x as f64 / scale, p.y as f64 / scale)
                                }
                                Position::Logical(p) => (p.x, p.y),
                            };
                            let tray_height = match rect.size {
                                Size::Physical(s) => {
                                    let scale = window.scale_factor().unwrap_or(1.0);
                                    s.height as f64 / scale
                                }
                                Size::Logical(s) => s.height,
                            };
                            let _ = window.set_position(Position::Logical(LogicalPosition::new(
                                tray_pos.0,
                                tray_pos.1 + tray_height,
                            )));
                        }
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)
        .expect("Failed to create tray icon");
}
