pub mod types;

use parking_lot::RwLock;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

use types::AppConfig;

pub struct ConfigManager {
    config: Arc<RwLock<AppConfig>>,
    path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("KeyMapper");
        fs::create_dir_all(&config_dir).ok();
        let path = config_dir.join("config.json");

        let config = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                    Ok(cfg) => {
                        info!("Config loaded from {:?}", path);
                        cfg
                    }
                    Err(e) => {
                        error!("Failed to parse config: {}", e);
                        AppConfig::default()
                    }
                },
                Err(e) => {
                    error!("Failed to read config: {}", e);
                    AppConfig::default()
                }
            }
        } else {
            info!("No config file found, using defaults");
            AppConfig::default()
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            path,
        }
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }

    pub fn update_config<F>(&self, f: F) -> AppConfig
    where
        F: FnOnce(&mut AppConfig),
    {
        {
            let mut cfg = self.config.write();
            f(&mut cfg);
        }
        self.save();
        self.get_config()
    }

    pub fn save(&self) {
        let cfg = self.config.read();
        match serde_json::to_string_pretty(&*cfg) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.path, json) {
                    error!("Failed to save config: {}", e);
                } else {
                    info!("Config saved to {:?}", self.path);
                }
            }
            Err(e) => error!("Failed to serialize config: {}", e),
        }
    }

    pub fn import_from_file(&self, file_path: &str) -> Result<AppConfig, String> {
        let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
        let new_config: AppConfig =
            serde_json::from_str(&content).map_err(|e| format!("Invalid config: {}", e))?;
        {
            let mut cfg = self.config.write();
            *cfg = new_config;
        }
        self.save();
        Ok(self.get_config())
    }

    pub fn export_to_file(&self, file_path: &str) -> Result<(), String> {
        let cfg = self.config.read();
        let json = serde_json::to_string_pretty(&*cfg).map_err(|e| e.to_string())?;
        fs::write(file_path, json).map_err(|e| e.to_string())
    }
}
