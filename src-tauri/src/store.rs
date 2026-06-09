use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdbDevice {
    pub id: String,
    pub name: String,
    pub ip_address: String,
    pub port: u16,
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "disconnected".to_string()
}

impl AdbDevice {
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip_address, self.port)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Store {
    #[serde(default)]
    pub devices: Vec<AdbDevice>,
    #[serde(default)]
    pub adb_path: Option<String>,
}

pub struct StoreManager {
    pub store: Mutex<Store>,
    path: PathBuf,
}

impl StoreManager {
    pub async fn new() -> Result<Self, String> {
        let data_dir = dirs::data_dir()
            .ok_or("Cannot determine app data directory")?
            .join("adbbar");
        fs::create_dir_all(&data_dir)
            .await
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
        let path = data_dir.join("devices.json");
        let store = Self::load_from(&path).await?;
        Ok(Self {
            store: Mutex::new(store),
            path,
        })
    }

    async fn load_from(path: &PathBuf) -> Result<Store, String> {
        if path.exists() {
            let content = fs::read_to_string(path)
                .await
                .map_err(|e| format!("Failed to read store: {}", e))?;
            let mut store: Store = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse store: {}", e))?;
            // Reset all statuses to disconnected on load
            for device in &mut store.devices {
                device.status = default_status();
            }
            Ok(store)
        } else {
            Ok(Store::default())
        }
    }

    pub async fn load(&self) -> Result<Store, String> {
        let store = Self::load_from(&self.path).await?;
        let mut guard = self.store.lock().await;
        *guard = store.clone();
        Ok(store)
    }

    pub async fn save(&self) -> Result<(), String> {
        let guard = self.store.lock().await;
        let content = serde_json::to_string_pretty(&*guard)
            .map_err(|e| format!("Failed to serialize store: {}", e))?;
        fs::write(&self.path, content)
            .await
            .map_err(|e| format!("Failed to write store: {}", e))
    }

    pub async fn add(&self, device: AdbDevice) -> Result<(), String> {
        let mut guard = self.store.lock().await;
        if guard.devices.iter().any(|d| d.address() == device.address()) {
            return Err(format!("Device {} already exists", device.address()));
        }
        guard.devices.push(device);
        drop(guard);
        self.save().await
    }

    pub async fn remove(&self, id: &str) -> Result<(), String> {
        let mut guard = self.store.lock().await;
        let before = guard.devices.len();
        guard.devices.retain(|d| d.id != id);
        if guard.devices.len() == before {
            return Err(format!("Device {} not found", id));
        }
        drop(guard);
        self.save().await
    }

    pub async fn clear(&self) -> Result<(), String> {
        let mut guard = self.store.lock().await;
        guard.devices.clear();
        drop(guard);
        self.save().await
    }

    pub async fn update_statuses(&self, status_map: &HashMap<String, String>) -> Result<(), String> {
        let mut guard = self.store.lock().await;
        for device in &mut guard.devices {
            let addr = device.address();
            if let Some(status) = status_map.get(&addr) {
                device.status = status.clone();
            } else {
                device.status = "disconnected".to_string();
            }
        }
        drop(guard);
        self.save().await
    }
}
