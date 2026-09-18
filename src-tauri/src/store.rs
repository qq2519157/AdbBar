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
    #[serde(default)]
    pub pinned: bool,
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
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub scrcpy_bitrate_mbps: Option<u32>,
    #[serde(default)]
    pub scrcpy_turn_screen_off: bool,
    #[serde(default)]
    pub scrcpy_max_size: Option<u32>,
    #[serde(default)]
    pub scrcpy_stay_awake: bool,
    #[serde(default)]
    pub scan_port: Option<u16>,
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

    pub async fn rename(&self, id: &str, new_name: &str) -> Result<(), String> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err("Device name cannot be empty".to_string());
        }
        let mut guard = self.store.lock().await;
        let device = guard
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("Device {} not found", id))?;
        device.name = new_name.to_string();
        drop(guard);
        self.save().await
    }

    pub async fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), String> {
        let mut guard = self.store.lock().await;
        let device = guard
            .devices
            .iter_mut()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("Device {} not found", id))?;
        device.pinned = pinned;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store_path(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "adbbar-store-test-{}-{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("devices.json")
    }

    fn sample_device() -> AdbDevice {
        AdbDevice {
            id: "192.168.1.5:5555".to_string(),
            name: "Old name".to_string(),
            ip_address: "192.168.1.5".to_string(),
            port: 5555,
            status: default_status(),
            pinned: false,
        }
    }

    #[tokio::test]
    async fn rename_updates_and_persists() {
        let path = temp_store_path("rename-ok");
        let manager = StoreManager {
            store: Mutex::new(Store::default()),
            path: path.clone(),
        };
        manager.add(sample_device()).await.unwrap();

        manager.rename("192.168.1.5:5555", "  Pixel 8  ").await.unwrap();

        let persisted = StoreManager::load_from(&path).await.unwrap();
        assert_eq!(persisted.devices.len(), 1);
        assert_eq!(persisted.devices[0].name, "Pixel 8");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn rename_rejects_unknown_id_and_blank_name() {
        let path = temp_store_path("rename-err");
        let manager = StoreManager {
            store: Mutex::new(Store::default()),
            path: path.clone(),
        };
        manager.add(sample_device()).await.unwrap();

        assert!(manager.rename("10.0.0.9:5555", "x").await.is_err());
        assert!(manager.rename("192.168.1.5:5555", "   ").await.is_err());

        let guard = manager.store.lock().await;
        assert_eq!(guard.devices[0].name, "Old name");
        drop(guard);
        let persisted = StoreManager::load_from(&path).await.unwrap();
        assert_eq!(persisted.devices[0].name, "Old name");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn set_pinned_updates_and_persists() {
        let path = temp_store_path("pin");
        let manager = StoreManager {
            store: Mutex::new(Store::default()),
            path: path.clone(),
        };
        manager.add(sample_device()).await.unwrap();

        manager.set_pinned("192.168.1.5:5555", true).await.unwrap();
        assert!(manager.set_pinned("missing:1", true).await.is_err());

        let persisted = StoreManager::load_from(&path).await.unwrap();
        assert!(persisted.devices[0].pinned);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn load_from_tolerates_missing_new_fields() {
        // devices.json written by an older app version (no pinned / options).
        let path = temp_store_path("legacy");
        std::fs::write(
            &path,
            r#"{"devices":[{"id":"10.0.0.2:5555","name":"Legacy","ip_address":"10.0.0.2","port":5555}]}"#,
        )
        .unwrap();
        let store = StoreManager::load_from(&path).await.unwrap();
        assert_eq!(store.devices.len(), 1);
        assert_eq!(store.devices[0].name, "Legacy");
        assert!(!store.devices[0].pinned);
        assert_eq!(store.devices[0].status, "disconnected");
        assert!(store.scan_port.is_none());
        let _ = std::fs::remove_file(&path);
    }
}
