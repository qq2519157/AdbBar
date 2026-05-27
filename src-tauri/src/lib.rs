mod adb;
mod scanner;
mod scrcpy;
mod store;

use adb::AdbService;
use scanner::{ScanProgress, ScanResult};
use scrcpy::{ScrcpyService, ScrcpyStatus};
use store::{AdbDevice, StoreManager};
use std::sync::Arc;
use tauri::{Emitter, Manager, RunEvent};

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
async fn refresh_all(state: tauri::State<'_, AppState>) -> Result<Vec<AdbDevice>, String> {
    adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await
}

#[tauri::command]
async fn scan_network(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    port: u16,
) -> Result<Vec<ScanResult>, String> {
    let app_handle = app.clone();
    let results = scanner::scan_subnet(port, move |progress: ScanProgress| {
        let _ = app_handle.emit("scan-progress", &progress);
    })
    .await?;

    // Auto-add found devices to store
    for result in &results {
        let device = AdbDevice {
            id: format!("{}:{}", result.ip, result.port),
            name: format!("ADB Device ({})", result.ip),
            ip_address: result.ip.clone(),
            port: result.port,
            status: "disconnected".to_string(),
        };
        let _ = state.store.add(device).await;
    }

    // Try to refresh statuses with adb
    let _ =
        adb::AdbService::refresh_statuses(state.adb.clone(), state.store.clone()).await;

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
async fn open_shell(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<(), String> {
    state.adb.open_shell(&address).await
}

#[tauri::command]
async fn launch_scrcpy(
    state: tauri::State<'_, AppState>,
    address: String,
) -> Result<(), String> {
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
async fn set_adb_path(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
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
async fn detect_scrcpy_status(
    state: tauri::State<'_, AppState>,
) -> Result<ScrcpyStatus, String> {
    Ok(state.scrcpy.detect().await)
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
            open_shell,
            launch_scrcpy,
            take_screenshot,
            install_apk,
            get_adb_path,
            set_adb_path,
            detect_scrcpy_status,
            install_scrcpy,
            quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::WindowEvent { event: tauri::WindowEvent::CloseRequested { api, .. }, label, .. } = event {
                if label == "main" {
                    // Hide window instead of closing
                    api.prevent_close();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }
        });
}
