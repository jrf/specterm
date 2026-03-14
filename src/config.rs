//! Configuration file support. Loads/saves from ~/.config/sonitus/config.toml.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mode: String,
    pub theme: String,
    pub fps: u64,
    pub bars: usize,
    pub low_freq: f32,
    pub high_freq: f32,
    pub smoothing: f32,
    pub monstercat: bool,
    pub noise_floor: f32,
    /// Number of distinct gradient colors (0 = use all stops in the theme).
    pub colors: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: "spectrum".to_string(),
            theme: "classic".to_string(),
            fps: 60,
            bars: 64,
            low_freq: 20.0,
            high_freq: 20000.0,
            smoothing: 0.5,
            monstercat: false,
            noise_floor: 0.0,
            colors: 0,
        }
    }
}

/// Get the config file path (~/.config/sonitus/config.toml).
pub fn config_path() -> PathBuf {
    dirs().join("config.toml")
}

fn dirs() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("sonitus")
}

/// Load config from disk, falling back to defaults for missing fields.
pub fn load() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Save config to disk.
pub fn save(config: &Config) -> std::io::Result<()> {
    let dir = dirs();
    fs::create_dir_all(&dir)?;
    let contents = toml::to_string_pretty(config).unwrap_or_default();
    fs::write(config_path(), contents)
}
