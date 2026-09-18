pub mod adb;
mod download;
pub mod locale;
mod scanner;
mod scrcpy;
pub mod store;
pub mod traymenu;

use adb::AdbService;
use scanner::{ScanProgress, ScanResult};
use scrcpy::{ScrcpyService, ScrcpyStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use store::{AdbDevice, StoreManager};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager, RunEvent};

static JUST_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn mark_just_shown() {
    JUST_SHOWN.store(true, Ordering::SeqCst);
}

pub fn build_tray_menu(
    app: &impl tauri::Manager<tauri::Wry>,
    devices: &[AdbDevice],
) -> Menu<tauri::Wry> {
    let sep = PredefinedMenuItem::separator(app).unwrap();
    let restart_adb =
        MenuItem::with_id(app, "restart-adb", locale::tray_text("restart_adb"), true, None::<&str>).unwrap();
    let enable_tcpip =
        MenuItem::with_id(app, "enable-tcpip", locale::tray_text("enable_tcpip"), true, None::<&str>)
            .unwrap();
    let disconnect_all = MenuItem::with_id(
        app,
        "disconnect-all",
        locale::tray_text("disconnect_all"),
        true,
        None::<&str>,
    )
    .unwrap();
    let quit = MenuItem::with_id(app, "quit", locale::tray_text("quit"), true, None::<&str>).unwrap();

    let connect_submenu = if devices.is_empty() {
        let empty =
            MenuItem::with_id(app, "no-devices", locale::tray_text("no_devices"), false, None::<&str>).unwrap();
        Submenu::with_items(app, locale::tray_text("quick_connect"), true, &[&empty]).unwrap()
    } else {
        let items: Vec<MenuItem<tauri::Wry>> = devices
            .iter()
            .map(|d| {
                let label = if d.status == "connected" {
                    format!("🟢 {} ({})", d.name, d.address())
                } else {
                    format!("{} ({})", d.name, d.address())
                };
                MenuItem::with_id(
                    app,
                    format!("connect:{}", d.address()),
                    label,
                    d.status != "connected",
                    None::<&str>,
                )
                .unwrap()
            })
            .collect();
        let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
        Submenu::with_items(app, locale::tray_text("quick_connect"), true, &refs).unwrap()
    };

    Menu::with_items(
        app,
        &[
            &connect_submenu,
            &sep,
            &restart_adb,
            &enable_tcpip,
            &disconnect_all,
            &sep,
            &quit,
        ],
    )
    .unwrap()
}

pub fn rebuild_tray_menu(app: &tauri::AppHandle, mut devices: Vec<AdbDevice>) {
    // Pinned devices first, keeping storage order otherwise (stable sort).
    devices.sort_by_key(|d| !d.pinned);
    let menu = build_tray_menu(app, &devices);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let connected = devices.iter().filter(|d| d.status == "connected").count();
        let _ = tray.set_tooltip(Some(locale::tray_tooltip(connected)));
        if traymenu::detached() {
            // macOS 27: attaching the menu to the status item makes AppKit
            // swallow the clicks; keep it for manual presentation instead.
            let state = app.state::<AppState>();
            *state.tray_menu.lock().unwrap() = Some(menu);
        } else {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

pub struct AppState {
    pub adb: Arc<AdbService>,
    pub store: Arc<StoreManager>,
    pub scrcpy: Arc<ScrcpyService>,
    /// Tray menu held aside on macOS 27, where the status item must not own it
    /// (see [`traymenu`]); presented manually on right-click.
    pub tray_menu: std::sync::Mutex<Option<Menu<tauri::Wry>>>,
}

#[tauri::command]
async fn get_devices(state: tauri::State<'_, AppState>) -> Result<Vec<AdbDevice>, String> {
    let guard = state.store.store.lock().await;
    Ok(guard.devices.clone())
}

async fn update_tray_menu(app: &tauri::AppHandle, state: &AppState) {
    let guard = state.store.store.lock().await;
    let devices = guard.devices.clone();
    drop(guard);
    crate::rebuild_tray_menu(app, devices);
}

#[tauri::command]
async fn connect_device(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    let result = state.adb.connect(&address).await?;
    let _ = adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await;
    update_tray_menu(&app, &state).await;
    Ok(result)
}

#[tauri::command]
async fn pair_device(
    state: tauri::State<'_, AppState>,
    address: String,
    code: String,
) -> Result<Option<String>, String> {
    let address = address.trim().to_string();
    let code = code.trim().to_string();
    if address.is_empty() {
        return Err("Pairing address cannot be empty".to_string());
    }
    if code.is_empty() {
        return Err("Pairing code cannot be empty".to_string());
    }
    state.adb.pair(&address, &code).await?;

    // The phone starts advertising a connect service right after pairing, but
    // registration can lag a couple of seconds — poll mDNS briefly for it and
    // return the connect address so the UI can add the device in one step.
    let ip_prefix = format!("{}:", address.split(':').next().unwrap_or_default());
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
        if let Ok(output) = state.adb.run(&["mdns", "services"], 5).await {
            let services = adb::parse_mdns_services(&output);
            if let Some(connect) = services
                .iter()
                .find(|s| s.kind == "connect" && s.address.starts_with(&ip_prefix))
            {
                return Ok(Some(connect.address.clone()));
            }
        }
    }
    Ok(None)
}

#[tauri::command]
async fn disconnect_device(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    let result = state.adb.disconnect(&address).await?;
    let _ = adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await;
    update_tray_menu(&app, &state).await;
    Ok(result)
}

/// Disconnect every stored device concurrently, then refresh statuses.
/// Shared by the `disconnect_all` command and the tray menu entry.
pub async fn disconnect_all_devices(
    adb: Arc<AdbService>,
    store: Arc<StoreManager>,
) -> Result<Vec<AdbDevice>, String> {
    let addresses = {
        let guard = store.store.lock().await;
        guard.devices.iter().map(|d| d.address()).collect::<Vec<_>>()
    };
    let mut joins = tokio::task::JoinSet::new();
    for addr in addresses {
        let adb = adb.clone();
        joins.spawn(async move {
            let _ = adb.disconnect(&addr).await;
        });
    }
    while joins.join_next().await.is_some() {}
    AdbService::refresh_statuses(adb, store).await
}

#[tauri::command]
async fn disconnect_all(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AdbDevice>, String> {
    let result =
        disconnect_all_devices(state.adb.clone(), state.store.clone()).await;
    update_tray_menu(&app, &state).await;
    result
}

#[tauri::command]
async fn refresh_all(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    reconnect: Option<bool>,
) -> Result<Vec<AdbDevice>, String> {
    if reconnect.unwrap_or(false) {
        let addresses = {
            let guard = state.store.store.lock().await;
            guard.devices.iter().map(|d| d.address()).collect::<Vec<_>>()
        };
        // Connect concurrently: unreachable hosts would otherwise serialize their
        // 10s timeouts and stall a refresh with several saved devices.
        let mut joins = tokio::task::JoinSet::new();
        for addr in addresses {
            let adb = state.adb.clone();
            joins.spawn(async move {
                let _ = adb.connect(&addr).await;
            });
        }
        while joins.join_next().await.is_some() {}
    }
    let result = adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await;
    update_tray_menu(&app, &state).await;
    result
}

#[tauri::command]
async fn scan_network(app: tauri::AppHandle, port: u16) -> Result<Vec<ScanResult>, String> {
    let app_handle = app.clone();
    let results = scanner::scan_subnet(port, move |progress: ScanProgress| {
        let _ = app_handle.emit("scan-progress", &progress);
    })
    .await?;

    Ok(results)
}

#[tauri::command]
async fn mdns_services(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<adb::MdnsService>, String> {
    let output = state.adb.run(&["mdns", "services"], 5).await?;
    Ok(adb::parse_mdns_services(&output))
}

#[tauri::command]
async fn add_device(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    ip_address: String,
    port: u16,
    name: String,
) -> Result<AdbDevice, String> {
    let device = AdbDevice {
        id: format!("{}:{}", ip_address, port),
        name,
        ip_address,
        port,
        status: "disconnected".to_string(),
        pinned: false,
    };
    let cloned = device.clone();
    state.store.add(device).await?;
    update_tray_menu(&app, &state).await;
    Ok(cloned)
}

#[tauri::command]
async fn remove_device(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    state.store.remove(&id).await?;
    update_tray_menu(&app, &state).await;
    Ok(())
}

#[tauri::command]
async fn rename_device(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), String> {
    state.store.rename(&id, &name).await?;
    update_tray_menu(&app, &state).await;
    Ok(())
}

#[tauri::command]
async fn clear_devices(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.store.clear().await?;
    update_tray_menu(&app, &state).await;
    Ok(())
}

#[tauri::command]
async fn export_devices(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let snapshot = {
        let guard = state.store.store.lock().await;
        let mut snapshot = guard.clone();
        // Keep the export portable: strip machine-local settings and runtime statuses.
        snapshot.adb_path = None;
        snapshot.locale = None;
        for device in &mut snapshot.devices {
            device.status = "disconnected".to_string();
        }
        snapshot
    };
    let content = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| format!("Failed to serialize devices: {}", e))?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
async fn import_devices(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<usize, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    let imported: store::Store =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse file: {}", e))?;
    let mut added = 0usize;
    {
        let mut guard = state.store.store.lock().await;
        for mut device in imported.devices {
            if guard.devices.iter().any(|d| d.address() == device.address()) {
                continue;
            }
            device.status = "disconnected".to_string();
            guard.devices.push(device);
            added += 1;
        }
    }
    state.store.save().await?;
    update_tray_menu(&app, &state).await;
    Ok(added)
}

#[tauri::command]
async fn open_shell(state: tauri::State<'_, AppState>, address: String) -> Result<(), String> {
    state.adb.open_shell(&address).await
}

#[tauri::command]
async fn launch_scrcpy(state: tauri::State<'_, AppState>, address: String) -> Result<(), String> {
    // Re-detect if path is not set
    {
        let guard = state.scrcpy.path.lock().await;
        if guard.is_none() {
            drop(guard);
            let status = state.scrcpy.detect().await;
            if !status.installed {
                return Err("scrcpy is not installed. Please install it first.".to_string());
            }
        }
    }
    let options = {
        let guard = state.store.store.lock().await;
        scrcpy::ScrcpyLaunchOptions {
            bitrate_mbps: guard.scrcpy_bitrate_mbps,
            turn_screen_off: guard.scrcpy_turn_screen_off,
            max_size: guard.scrcpy_max_size,
            stay_awake: guard.scrcpy_stay_awake,
        }
    };
    state.scrcpy.launch(&address, options).await
}

#[tauri::command]
async fn set_scrcpy_options(
    state: tauri::State<'_, AppState>,
    bitrate_mbps: Option<u32>,
    turn_screen_off: bool,
    max_size: Option<u32>,
    stay_awake: bool,
) -> Result<(), String> {
    if let Some(mbps) = bitrate_mbps {
        if !(1..=1000).contains(&mbps) {
            return Err("Bitrate must be 1-1000 Mbps".to_string());
        }
    }
    if let Some(size) = max_size {
        if !(100..=8192).contains(&size) {
            return Err("Max size must be 100-8192 px".to_string());
        }
    }
    {
        let mut guard = state.store.store.lock().await;
        guard.scrcpy_bitrate_mbps = bitrate_mbps;
        guard.scrcpy_turn_screen_off = turn_screen_off;
        guard.scrcpy_max_size = max_size;
        guard.scrcpy_stay_awake = stay_awake;
    }
    state.store.save().await
}

#[tauri::command]
async fn get_scrcpy_options(
    state: tauri::State<'_, AppState>,
) -> Result<(Option<u32>, bool, Option<u32>, bool), String> {
    let guard = state.store.store.lock().await;
    Ok((
        guard.scrcpy_bitrate_mbps,
        guard.scrcpy_turn_screen_off,
        guard.scrcpy_max_size,
        guard.scrcpy_stay_awake,
    ))
}

#[tauri::command]
async fn get_scan_port(state: tauri::State<'_, AppState>) -> Result<Option<u16>, String> {
    let guard = state.store.store.lock().await;
    Ok(guard.scan_port)
}

#[tauri::command]
async fn set_scan_port(state: tauri::State<'_, AppState>, port: u16) -> Result<(), String> {
    if !(1..=65535).contains(&port) {
        return Err("Port must be 1-65535".to_string());
    }
    {
        let mut guard = state.store.store.lock().await;
        guard.scan_port = Some(port);
    }
    state.store.save().await
}

#[tauri::command]
async fn set_device_pinned(
    state: tauri::State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    state.store.set_pinned(&id, pinned).await
}

#[tauri::command]
async fn take_screenshot(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    state.adb.take_screenshot(&address).await
}

#[tauri::command]
async fn get_device_props(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let output = state.adb.run(&["-s", &address, "shell", "getprop"], 10).await?;
    Ok(adb::parse_getprop_output(&output))
}

#[tauri::command]
async fn install_apk(
    state: tauri::State<'_, AppState>,
    address: String,
    apk_path: String,
) -> Result<String, String> {
    state.adb.install_apk(&address, &apk_path).await
}

#[tauri::command]
async fn get_adb_path(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.adb.get_adb_path().await)
}

#[tauri::command]
async fn detect_adb_path() -> Result<String, String> {
    let path = AdbService::detect_adb_path()
        .ok_or_else(|| "Could not detect ADB in PATH or common install locations".to_string())?;
    AdbService::validate_adb_path(&path).await?;
    Ok(path)
}

#[tauri::command]
async fn set_adb_path(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let path = path.trim().to_string();
    AdbService::validate_adb_path(&path).await?;
    state.adb.set_adb_path(path.clone()).await;

    // Persist the new path in the store
    {
        let mut guard = state.store.store.lock().await;
        guard.adb_path = Some(path);
    }
    state.store.save().await?;

    Ok(())
}

#[tauri::command]
async fn install_adb(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let app_handle = app.clone();
    let path = state
        .adb
        .install(move |msg: String| {
            let _ = app_handle.emit("adb-install-progress", &msg);
        })
        .await?;

    // Persist the installed path in the store (mirrors set_adb_path)
    {
        let mut guard = state.store.store.lock().await;
        guard.adb_path = Some(path.clone());
    }
    state.store.save().await?;

    Ok(path)
}

#[tauri::command]
async fn check_adb_path(path: String) -> Result<(), String> {
    AdbService::validate_adb_path(path.trim()).await
}

#[tauri::command]
async fn detect_scrcpy_status(state: tauri::State<'_, AppState>) -> Result<ScrcpyStatus, String> {
    Ok(state.scrcpy.detect().await)
}

#[tauri::command]
async fn set_scrcpy_path(state: tauri::State<'_, AppState>, path: String) -> Result<(), String> {
    let path = path.trim().to_string();
    ScrcpyService::validate_path(&path).await?;
    state.scrcpy.set_path(path).await;
    Ok(())
}

#[tauri::command]
async fn install_scrcpy(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let app_handle = app.clone();
    state
        .scrcpy
        .install(move |msg: String| {
            let _ = app_handle.emit("scrcpy-install-progress", &msg);
        })
        .await
}

#[tauri::command]
async fn restart_adb(state: tauri::State<'_, AppState>) -> Result<String, String> {
    state.adb.run(&["kill-server"], 5).await?;
    state.adb.run(&["start-server"], 10).await?;
    Ok("ADB server restarted".to_string())
}

#[tauri::command]
async fn enable_tcpip(
    state: tauri::State<'_, AppState>,
    address: Option<String>,
    port: Option<u16>,
) -> Result<String, String> {
    let port_str = port.unwrap_or(5555).to_string();
    match address {
        Some(addr) => state.adb.run(&["-s", &addr, "tcpip", &port_str], 10).await,
        None => state.adb.run(&["tcpip", &port_str], 10).await,
    }
}

#[tauri::command]
async fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
async fn get_locale() -> Result<String, String> {
    Ok(locale::current_locale().to_string())
}

#[tauri::command]
async fn set_locale(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    locale: String,
) -> Result<(), String> {
    let locale = match locale.as_str() {
        "en" | "zh" => locale,
        _ => return Err("Invalid locale".to_string()),
    };
    // Persist
    {
        let mut guard = state.store.store.lock().await;
        guard.locale = Some(locale.clone());
    }
    state.store.save().await?;
    // Update runtime locale and rebuild tray menu
    crate::locale::set_locale(&locale);
    update_tray_menu(&app, &state).await;
    Ok(())
}

/// Build and run the Tauri app with a custom setup callback.
/// The callback receives the `tauri::App` after plugins and state are initialized,
/// allowing the caller (main.rs) to set up the system tray.
pub fn run_builder<F>(tray_setup: F)
where
    F: FnOnce(&tauri::App) + Send + 'static,
{
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            // Initialize the persistent store
            let store_manager = tauri::async_runtime::block_on(StoreManager::new())
                .expect("Failed to initialize store");

            let configured_path = {
                let guard = tauri::async_runtime::block_on(store_manager.store.lock());
                guard.adb_path.clone()
            };

            let configured_locale = {
                let guard = tauri::async_runtime::block_on(store_manager.store.lock());
                guard.locale.clone()
            };
            locale::init_locale(configured_locale.as_deref());

            let adb_service = AdbService::new(configured_path);
            let scrcpy_service = ScrcpyService::new();

            app.manage(AppState {
                adb: Arc::new(adb_service),
                store: Arc::new(store_manager),
                scrcpy: Arc::new(scrcpy_service),
                tray_menu: std::sync::Mutex::new(None),
            });

            // Call the caller's setup (tray icon creation, etc.)
            tray_setup(app);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            connect_device,
            pair_device,
            disconnect_device,
            disconnect_all,
            refresh_all,
            scan_network,
            mdns_services,
            get_scan_port,
            set_scan_port,
            add_device,
            remove_device,
            rename_device,
            clear_devices,
            export_devices,
            import_devices,
            open_shell,
            launch_scrcpy,
            set_scrcpy_options,
            get_scrcpy_options,
            set_device_pinned,
            take_screenshot,
            get_device_props,
            install_apk,
            get_adb_path,
            detect_adb_path,
            set_adb_path,
            install_adb,
            check_adb_path,
            detect_scrcpy_status,
            set_scrcpy_path,
            install_scrcpy,
            restart_adb,
            enable_tcpip,
            hide_window,
            quit_app,
            get_locale,
            set_locale,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::WindowEvent {
                event: tauri::WindowEvent::CloseRequested { api, .. },
                label,
                ..
            } if label == "main" => {
                api.prevent_close();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            RunEvent::WindowEvent {
                event: tauri::WindowEvent::Focused(false),
                label,
                ..
            } if label == "main" => {
                if JUST_SHOWN.swap(false, Ordering::SeqCst) {
                    // Ignore the first focus-loss after showing the window
                } else if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            _ => {}
        });
}
