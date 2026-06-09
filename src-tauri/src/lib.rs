mod adb;
mod scanner;
mod scrcpy;
mod store;

use adb::AdbService;
use scanner::{ScanProgress, ScanResult};
use scrcpy::{ScrcpyService, ScrcpyStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use store::{AdbDevice, StoreManager};
use tauri::{Emitter, Manager, RunEvent};

static JUST_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn mark_just_shown() {
    JUST_SHOWN.store(true, Ordering::SeqCst);
}

pub struct AppState {
    pub adb: Arc<AdbService>,
    pub store: Arc<StoreManager>,
    pub scrcpy: Arc<ScrcpyService>,
}

#[tauri::command]
async fn get_devices(state: tauri::State<'_, AppState>) -> Result<Vec<AdbDevice>, String> {
    let guard = state.store.store.lock().await;
    Ok(guard.devices.clone())
}

#[tauri::command]
async fn connect_device(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    let result = state.adb.connect(&address).await?;

    // Refresh statuses after connect
    let _ = adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await;

    Ok(result)
}

#[tauri::command]
async fn disconnect_device(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    let result = state.adb.disconnect(&address).await?;

    // Refresh statuses after disconnect
    let _ = adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await;

    Ok(result)
}

#[tauri::command]
async fn refresh_all(
    state: tauri::State<'_, AppState>,
    reconnect: Option<bool>,
) -> Result<Vec<AdbDevice>, String> {
    if reconnect.unwrap_or(false) {
        // Re-attempt connection for all stored devices to detect dropped connections.
        let guard = state.store.store.lock().await;
        let addresses: Vec<String> = guard.devices.iter().map(|d| d.address()).collect();
        drop(guard);
        for addr in &addresses {
            let _ = state.adb.connect(addr).await;
        }
    }
    adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await
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
async fn add_device(
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
    };
    let cloned = device.clone();
    state.store.add(device).await?;
    Ok(cloned)
}

#[tauri::command]
async fn remove_device(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    state.store.remove(&id).await
}

#[tauri::command]
async fn clear_devices(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.store.clear().await
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
    state.scrcpy.launch(&address).await
}

#[tauri::command]
async fn take_screenshot(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    state.adb.take_screenshot(&address).await
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
async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
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

            let adb_service = AdbService::new(configured_path);
            let scrcpy_service = ScrcpyService::new();

            app.manage(AppState {
                adb: Arc::new(adb_service),
                store: Arc::new(store_manager),
                scrcpy: Arc::new(scrcpy_service),
            });

            // Call the caller's setup (tray icon creation, etc.)
            tray_setup(app);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            connect_device,
            disconnect_device,
            refresh_all,
            scan_network,
            add_device,
            remove_device,
            clear_devices,
            open_shell,
            launch_scrcpy,
            take_screenshot,
            install_apk,
            get_adb_path,
            detect_adb_path,
            set_adb_path,
            detect_scrcpy_status,
            set_scrcpy_path,
            install_scrcpy,
            restart_adb,
            enable_tcpip,
            quit_app,
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
