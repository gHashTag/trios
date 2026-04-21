//! Configuration management for tri CLI
//!
//! Reads from `.trinity/tri.toml` with agent settings, paths, and preferences.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration loaded from `.trinity/tri.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Current NATO agent code (ALFA, BRAVO, CHARLIE, DELTA, ECHO)
    #[serde(default = "default_nato")]
    pub agent_nato: String,

    /// Default experiment base path
    #[serde(default)]
    pub exp_base_path: Option<String>,

    /// GitHub repo owner
    #[serde(default = "default_owner")]
    pub repo_owner: String,

    /// GitHub repo name
    #[serde(default = "default_repo")]
    pub repo_name: String,

    /// Default training steps
    #[serde(default = "default_steps")]
    pub training_steps: usize,

    /// Default batch size
    #[serde(default = "default_batch")]
    pub batch_size: usize,

    /// Default seed
    #[serde(default = "default_seed")]
    pub seed: u64,

    /// CPU-only mode (L10)
    #[serde(default)]
    pub cpu_only: bool,
}

fn default_nato() -> String {
    "ALFA".to_string()
}

fn default_owner() -> String {
    "gHashTag".to_string()
}

fn default_repo() -> String {
    "trios".to_string()
}

fn default_steps() -> usize {
    300
}

fn default_batch() -> usize {
    32
}

fn default_seed() -> u64 {
    42
}

impl Config {
    /// Load config from `.trinity/tri.toml`
    pub fn load() -> Result<Self> {
        let path = Self::path();

        if !path.exists() {
            let default = Config::default();
            default.save()?;
            return Ok(default);
        }

        let content = std::fs::read_to_string(&path)
            .context("Failed to read tri.toml")?;

        toml::from_str(&content).context("Failed to parse tri.toml")
    }

    /// Save config to `.trinity/tri.toml`
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        std::fs::write(&path, content)
            .context("Failed to write tri.toml")?;

        Ok(())
    }

    /// Get config file path
    fn path() -> PathBuf {
        PathBuf::from(".trinity/tri.toml")
    }

    /// Get current agent full name from NATO code
    pub fn agent_name(&self) -> &'static str {
        match self.agent_nato.as_str() {
            "ALFA" => "FOXTROT",
            "BRAVO" => "INDIGO",
            "CHARLIE" => "JULIETT",
            "DELTA" => "KILO",
            "ECHO" => "LIMA",
            _ => "UNKNOWN",
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_nato: default_nato(),
            exp_base_path: None,
            repo_owner: default_owner(),
            repo_name: default_repo(),
            training_steps: default_steps(),
            batch_size: default_batch(),
            seed: default_seed(),
            cpu_only: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let cfg = Config::default();
        assert_eq!(cfg.agent_nato, "ALFA");
        assert_eq!(cfg.repo_owner, "gHashTag");
        assert_eq!(cfg.repo_name, "trios");
    }

    #[test]
    fn test_agent_names() {
        let mut cfg = Config::default();
        cfg.agent_nato = "BRAVO".to_string();
        assert_eq!(cfg.agent_name(), "INDIGO");
    }
}
