use serde::{Deserialize, Serialize};
use std::fs;
use crate::paths;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub name: String,
    pub port: u16,
    #[serde(default = "default_font_size")]
    pub font_size: u8,
    #[serde(default)]
    pub notify_target: String,
    #[serde(default)]
    pub accept_transfers: bool,
    #[serde(default)]
    pub transfer_from: String,
    #[serde(default = "default_true")]
    pub auto_bump_serial: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_font_size")]
    pub font_size: u8,
}

fn default_font_size() -> u8 {
    16
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config { name: String::new(), port: 1053, font_size: 16, notify_target: String::new(), accept_transfers: false, transfer_from: String::new(), auto_bump_serial: true }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig { font_size: 16 }
    }
}

pub fn read_config(identity: &str) -> Config {
    let path = paths::config_path(identity);
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(identity: &str, config: &Config) -> std::io::Result<()> {
    let path = paths::config_path(identity);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

pub fn read_app_config() -> AppConfig {
    let path = paths::app_config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_app_config(config: &AppConfig) -> std::io::Result<()> {
    let path = paths::app_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_port_1053() {
        let config = Config::default();
        assert_eq!(config.port, 1053);
    }

    #[test]
    fn config_roundtrips_through_json() {
        let config = Config { port: 5353, ..Config::default() };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.port, 5353);
    }

    #[test]
    fn invalid_json_returns_default() {
        let parsed: Config = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(parsed.port, 1053);
    }

    #[test]
    fn partial_json_returns_default() {
        let result: Result<Config, _> = serde_json::from_str("{\"unknown\": true}");
        let config = result.unwrap_or_default();
        assert_eq!(config.port, 1053);
    }
}
