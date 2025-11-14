// Configuration management for PanPipe
// Handles loading/saving settings, with sensible defaults when config is missing

use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub music_directories: Vec<PathBuf>,
    pub database_path: PathBuf,
    pub spotify: SpotifyConfig,
    pub behavior: BehaviorConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyConfig {
    pub client_id: Option<String>,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub skip_threshold_seconds: u64,
    pub weight_decay_days: u64,
    pub min_play_time_for_tracking: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub show_notifications: bool,
    pub notification_duration_ms: u64,
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("panpipe");
        
        Self {
            music_directories: vec![
                dirs::audio_dir().unwrap_or_else(|| PathBuf::from("~/Music")),
            ],
            database_path: config_dir.join("panpipe.db"),
            spotify: SpotifyConfig {
                client_id: None,
                redirect_uri: "http://localhost:8888/callback".to_string(),
            },
            behavior: BehaviorConfig {
                skip_threshold_seconds: 30,
                weight_decay_days: 30,
                min_play_time_for_tracking: 10,
            },
            ui: UiConfig {
                show_notifications: true,
                notification_duration_ms: 3000,
                theme: "default".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        
        Ok(())
    }
    
    fn config_path() -> Result<PathBuf> {
        let config_dir = config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("panpipe");
        
        Ok(config_dir.join("config.toml"))
    }
}
