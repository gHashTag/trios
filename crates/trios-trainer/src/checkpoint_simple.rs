//! Simplified checkpoint saving for Phase P0 Audit
//!
//! Minimal implementation for Phase P0 — no complex serialization, just BPB tracking.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use anyhow::Result;
use crate::model::ModelParameters;

/// Simple checkpoint structure for Phase P0
#[derive(Debug, Clone)]
pub struct SimpleCheckpoint {
    pub step: usize,
    pub bpb: f32,
    pub best_bpb: f32,
    pub seed: u64,
}

impl SimpleCheckpoint {
    pub fn new(step: usize, bpb: f32, best_bpb: f32, seed: u64) -> Self {
        Self {
            step,
            bpb,
            best_bpb,
            seed,
        }
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        // Create checkpoint file name
        let filename = format!("checkpoint_step_{:05}.txt", self.step);
        let path = dir.join(&filename);

        // Write simple text format
        let mut file = File::create(&path)?;
        writeln!(file, "# Phase P0 Checkpoint")?;
        writeln!(file, "step = {}", self.step)?;
        writeln!(file, "bpb = {:.4}", self.bpb)?;
        writeln!(file, "best_bpb = {:.4}", self.best_bpb)?;
        writeln!(file, "seed = {}", self.seed)?;
        file.flush()?;

        println!("Saved checkpoint to {}", path.display());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let ckpt = SimpleCheckpoint::new(1000, 2.5, 2.3, 42);
        assert_eq!(ckpt.step, 1000);
        assert_eq!(ckpt.bpb, 2.5);
        assert_eq!(ckpt.best_bpb, 2.3);
        assert_eq!(ckpt.seed, 42);
    }

    #[test]
    fn test_checkpoint_save() {
        let ckpt = SimpleCheckpoint::new(1000, 2.5, 2.3, 42);
        let dir = PathBuf::from("/tmp");
        ckpt.save(&dir).unwrap();

        let path = dir.join("checkpoint_step_01000.txt");
        assert!(path.exists());
    }
}
