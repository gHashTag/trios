//! Configuration loading and validation
//!
//! Loads TOML configs from files and/or environment variables.
//! Enforces INV-8: lr in phi-band [0.001, 0.01].

use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub training: TrainingConfig,
    pub model: ModelConfig,
    pub jepa: Option<JepaConfig>,
    pub ledger: LedgerConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingConfig {
    /// Seed for reproducibility
    pub seed: u64,

    /// Total training steps
    pub steps: usize,

    /// Batch size
    pub batch_size: usize,

    /// Learning rate (INV-8: must be in [0.001, 0.01])
    #[serde(default = "default_lr")]
    pub lr: f32,

    /// Checkpoint interval in steps
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval: usize,

    /// Evaluation interval in steps
    #[serde(default = "default_eval_interval")]
    pub eval_interval: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    /// Model dimension
    pub d_model: usize,

    /// Number of attention layers
    pub n_layers: usize,

    /// Context length
    pub context_len: usize,

    /// Feedforward dimension multiplier
    pub ff_mult: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JepaConfig {
    /// JEPA mask ratio
    pub mask_ratio: f32,

    /// JEPA EMA decay
    pub ema_decay: f32,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerConfig {
    /// Path to ledger file (seed_results.jsonl)
    pub path: String,

    /// Whether to push rows back to repo
    #[serde(default)]
    pub push_to_repo: bool,

    /// Repo for ledger push (if push_to_repo is true)
    pub repo_url: Option<String>,
}

fn default_lr() -> f32 { 0.004 } // alpha_phi / phi^3 (INV-8)
fn default_checkpoint_interval() -> usize { 1000 }
fn default_eval_interval() -> usize { 500 }

impl Config {
    /// Load config from TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, LoadConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| LoadConfigError::ReadError(e.to_string()))?;

        let mut config: Config = toml::from_str(&content)
            .map_err(|e| LoadConfigError::ParseError(e.to_string()))?;

        // Validate INV-8: lr in phi-band
        if !(0.001..=0.01).contains(&config.training.lr) {
            return Err(LoadConfigError::InvalidLr(config.training.lr));
        }

        // Override from env vars
        if let Ok(seed) = std::env::var("TRIOS_SEED") {
            config.training.seed = seed.parse()
                .map_err(|_| LoadConfigError::InvalidEnvVar("TRIOS_SEED".into()))?;
        }

        if let Ok(steps) = std::env::var("TRIOS_STEPS") {
            config.training.steps = steps.parse()
                .map_err(|_| LoadConfigError::InvalidEnvVar("TRIOS_STEPS".into()))?;
        }

        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(String),

    #[error("Failed to parse config: {0}")]
    ParseError(String),

    #[error("LR {0} violates INV-8: must be in [0.001, 0.01]")]
    InvalidLr(f32),

    #[error("Invalid env var {0}")]
    InvalidEnvVar(String),
}

/// INV-8: lr in phi-band [0.001, 0.01] (proven by lr_convergence.v)
pub fn validate_lr_phi_band(lr: f32) -> bool {
    (0.001..=0.01).contains(&lr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_lr_phi_band() {
        assert!(validate_lr_phi_band(0.004));
        assert!(validate_lr_phi_band(0.001));
        assert!(validate_lr_phi_band(0.01));
        assert!(!validate_lr_phi_band(0.0009));
        assert!(!validate_lr_phi_band(0.011));
    }
}
