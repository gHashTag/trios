//! ASHA (Asynchronous Successive Halving Algorithm) implementation
//!
//! Trinity-optimized: rungs at 1k → 3k → 9k → 27k (3^k progression)

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// ASHA rungs (Trinity 3^k progression)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AshaRung {
    Rung1000 = 1000,
    Rung3000 = 3000,
    Rung9000 = 9000,
    Rung27000 = 27000,
}

impl AshaRung {
    /// Get all rungs in order
    pub fn all() -> Vec<AshaRung> {
        vec![
            AshaRung::Rung1000,
            AshaRung::Rung3000,
            AshaRung::Rung9000,
            AshaRung::Rung27000,
        ]
    }

    /// Get next rung after current
    pub fn next(&self) -> Option<AshaRung> {
        match self {
            AshaRung::Rung1000 => Some(AshaRung::Rung3000),
            AshaRung::Rung3000 => Some(AshaRung::Rung9000),
            AshaRung::Rung9000 => Some(AshaRung::Rung27000),
            AshaRung::Rung27000 => None,
        }
    }

    /// Get step value
    pub fn step(&self) -> usize {
        *self as usize
    }

    /// Get rung as i32 for database
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// ASHA trial configuration
#[derive(Debug, Clone)]
pub struct AshaConfig {
    /// Target BPB to declare winner
    pub target_bpb: f64,

    /// Top fraction to keep at each rung (e.g., 0.33 = top 33%)
    pub keep_fraction: f64,

    /// Minimum trials to start pruning
    pub min_trials: usize,

    /// Whether to run infinite loop (for continuous search)
    pub continuous: bool,
}

impl Default for AshaConfig {
    fn default() -> Self {
        Self {
            target_bpb: 1.5,  // IGLA target
            keep_fraction: 0.33,
            min_trials: 10,
            continuous: true,
        }
    }
}

/// Parse BPB from subprocess stdout
pub fn parse_bpb(output: &Result<std::process::Output, std::io::Error>) -> Result<f64> {
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .rev()
                .find(|l| l.starts_with("BPB="))
                .and_then(|l| l.trim_start_matches("BPB=").parse().ok())
                .ok_or_else(|| anyhow::anyhow!("No BPB found in output"))
        }
        Err(e) => Err(anyhow::anyhow!("Subprocess error: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rung_progression() {
        assert_eq!(AshaRung::Rung1000.next(), Some(AshaRung::Rung3000));
        assert_eq!(AshaRung::Rung3000.next(), Some(AshaRung::Rung9000));
        assert_eq!(AshaRung::Rung9000.next(), Some(AshaRung::Rung27000));
        assert_eq!(AshaRung::Rung27000.next(), None);
    }

    #[test]
    fn test_rung_steps() {
        assert_eq!(AshaRung::Rung1000.step(), 1000);
        assert_eq!(AshaRung::Rung3000.step(), 3000);
        assert_eq!(AshaRung::Rung9000.step(), 9000);
        assert_eq!(AshaRung::Rung27000.step(), 27000);
    }

    #[test]
    fn test_asha_config_default() {
        let config = AshaConfig::default();
        assert_eq!(config.target_bpb, 1.5);
        assert_eq!(config.keep_fraction, 0.33);
        assert!(config.continuous);
    }

    #[test]
    fn test_parse_bpb_success() {
        let stdout = b"Training...\nBPB=2.5329\nDone";
        let output = Ok(std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: stdout.to_vec(),
            stderr: vec![],
        });
        let bpb = parse_bpb(&output).unwrap();
        assert!((bpb - 2.5329).abs() < 0.0001);
    }

    #[test]
    fn test_parse_bpb_failure() {
        let stdout = b"Training...\nDone";
        let output = Ok(std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: stdout.to_vec(),
            stderr: vec![],
        });
        assert!(parse_bpb(&output).is_err());
    }
}
