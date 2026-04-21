use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_seeds: u32,
    pub trainer_path: Option<String>,
    pub db_path: PathBuf,
    pub lock_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_seeds: 1,
            trainer_path: None,
            db_path: PathBuf::from(".trinity/leaderboard.db"),
            lock_path: PathBuf::from(".trinity/tri.lock"),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = PathBuf::from(".trinity/tri.toml");
        
        if !path.exists() {
            return Ok(Config::default());
        }

        let contents = std::fs::read_to_string(&path)?;
        let mut config: Config = toml::from_str(&contents)?;
        
        // Resolve relative paths
        if !config.db_path.is_absolute() {
            config.db_path = PathBuf::from(".trinity").join(&config.db_path);
        }
        if !config.lock_path.is_absolute() {
            config.lock_path = PathBuf::from(".trinity").join(&config.lock_path);
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = PathBuf::from(".trinity/tri.toml");
        std::fs::create_dir_all(path.parent().unwrap())?;
        let toml = toml::to_string_pretty(self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}
