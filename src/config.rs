use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub prompt: String,
    pub history_size: usize,
    pub enable_colors: bool,
    pub aliases: std::collections::HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: "➜ {cwd} $ ".to_string(),
            history_size: 1000,
            enable_colors: true,
            aliases: std::collections::HashMap::new(),
        }
    }
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(config_path) = path {
            if config_path.exists() {
                let content = std::fs::read_to_string(config_path)?;
                let config: Config = toml::from_str(&content)?;
                Ok(config)
            } else {
                eprintln!("Config file not found at {:?}, using defaults", config_path);
                Ok(Config::default())
            }
        } else {
            // Try to load from default locations
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let default_config = Path::new(&home_dir).join(".wsh.toml");

            if default_config.exists() {
                let content = std::fs::read_to_string(&default_config)?;
                let config: Config = toml::from_str(&content)?;
                Ok(config)
            } else {
                Ok(Config::default())
            }
        }
    }

    /* pub fn save(&self, path: &Path) -> Result<()> {  // for future -__-
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    } */
}
