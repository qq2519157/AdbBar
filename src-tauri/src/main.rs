#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    image::Image,
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
    let tray_icon = tray_icon_image();

    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)
        .tooltip("ADB Bar")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
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
                            let tray_width = match rect.size {
                                Size::Physical(s) => s.width as f64 / scale,
                                Size::Logical(s) => s.width,
                            };
                            let win_size = window
                                .inner_size()
                                .unwrap_or_else(|_| tauri::PhysicalSize::new(320, 480));
                            let win_w = win_size.width as f64 / scale;
                            let tray_center_x = tray_x + tray_width / 2.0;
                            #[cfg(target_os = "windows")]
                            let win_h = win_size.height as f64 / scale;
                            #[cfg(target_os = "macos")]
                            let y = tray_y + tray_height;
                            #[cfg(target_os = "windows")]
                            let y = tray_y - win_h;
                            let _ = window
                                .set_position(LogicalPosition::new(tray_center_x - win_w / 2.0, y));
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

fn tray_icon_image() -> Image<'static> {
    Image::new(include_bytes!("../icons/tray-adb.rgba"), 22, 22).to_owned()
}
