//! SR-HACK-00: Glossary
//!
//! Trinity IGLA Race ecosystem terminology.
//! Dep-free, serde-serializable types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unified glossary term type for IGLA Race system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Term {
    /// End-to-end test-time training pipeline with O(1) complexity per chunk
    PipelineO1,
    /// Entry point for algorithm spec (train_gpt.py path, hash, env vars)
    AlgorithmEntry,
    /// Execution lane (scarab, strategy-queue, trainer-runner, bpb-writer, gardener, railway-deployer)
    Lane,
    /// L-f3 / L-f4 falsifier checkpoint
    Gate,
    /// Ring hierarchy (SR, MR, LR)
    RingTier,
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::PipelineO1 => write!(f, "PipelineO1"),
            Term::AlgorithmEntry => write!(f, "AlgorithmEntry"),
            Term::Lane => write!(f, "Lane"),
            Term::Gate => write!(f, "Gate"),
            Term::RingTier => write!(f, "RingTier"),
        }
    }
}

/// Represents an algorithm entry point with metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlgorithmEntry {
    /// Path to train_gpt.py payload (lives in parameter-golf/records/, not in crates/)
    pub entry_path: String,
    /// SHA-256 hash of entry file
    pub entry_hash: [u8; 32],
    /// Environment variables for this algorithm
    pub env: Vec<(String, String)>,
    /// Baseline golden state hash (optional)
    pub golden_state_hash: Option<[u8; 32]>,
    /// Coq theorem reference (optional)
    pub theorem: Option<String>,
}

impl AlgorithmEntry {
    /// Create a new algorithm entry
    pub fn new(
        entry_path: String,
        entry_hash: [u8; 32],
        env: Vec<(String, String)>,
    ) -> Self {
        Self {
            entry_path,
            entry_hash,
            env,
            golden_state_hash: None,
            theorem: None,
        }
    }

    /// Validate entry hash format
    pub fn validate_hash(&self) -> Result<(), String> {
        if self.entry_hash.len() != 32 {
            return Err("Entry hash must be 32 bytes".to_string());
        }
        Ok(())
    }
}

/// Represents an execution lane in the pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lane {
    /// Scarab job claiming
    Scarab,
    /// Strategy queue management
    StrategyQueue,
    /// Trainer runner execution
    TrainerRunner,
    /// BPB writer persistence
    BpbWriter,
    /// Gardener pruning and victory checking
    Gardener,
    /// Railway deployer deployment
    RailwayDeployer,
}

impl fmt::Display for Lane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lane::Scarab => write!(f, "Scarab"),
            Lane::StrategyQueue => write!(f, "StrategyQueue"),
            Lane::TrainerRunner => write!(f, "TrainerRunner"),
            Lane::BpbWriter => write!(f, "BpbWriter"),
            Lane::Gardener => write!(f, "Gardener"),
            Lane::RailwayDeployer => write!(f, "RailwayDeployer"),
        }
    }
}

/// Represents an L-f3 / L-f4 falsifier checkpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    /// Gate identifier
    pub name: String,
    /// Gate type (falsifier)
    pub gate_type: GateType,
    /// Whether gate is active
    pub active: bool,
}

/// Types of falsifier gates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateType {
    /// L-f3: BPB convergence gate
    Lf3BpbConvergence,
    /// L-f4: Parameter range gate
    Lf4ParameterRange,
    /// Custom falsifier
    Custom(String),
}

impl fmt::Display for Gate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gate({})", self.name)
    }
}

impl Gate {
    /// Create a new gate
    pub fn new(name: String, gate_type: GateType) -> Self {
        Self {
            name,
            gate_type,
            active: true,
        }
    }

    /// Deactivate this gate
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Activate this gate
    pub fn activate(&mut self) {
        self.active = true;
    }
}

/// Ring tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RingTier {
    /// Short Ring: 7-day cycle
    SR,
    /// Medium Ring: 30-day cycle
    MR,
    /// Long Ring: 90-day cycle
    LR,
}

impl fmt::Display for RingTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RingTier::SR => write!(f, "SR (7 days)"),
            RingTier::MR => write!(f, "MR (30 days)"),
            RingTier::LR => write!(f, "LR (90 days)"),
        }
    }
}

impl RingTier {
    /// Get cycle duration in days
    pub fn duration_days(&self) -> u32 {
        match self {
            RingTier::SR => 7,
            RingTier::MR => 30,
            RingTier::LR => 90,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_display() {
        assert_eq!(format!("{}", Term::PipelineO1), "PipelineO1");
        assert_eq!(format!("{}", Term::AlgorithmEntry), "AlgorithmEntry");
        assert_eq!(format!("{}", Term::Lane), "Lane");
    }

    #[test]
    fn test_lane_display() {
        assert_eq!(format!("{}", Lane::Scarab), "Scarab");
        assert_eq!(format!("{}", Lane::Gardener), "Gardener");
    }

    #[test]
    fn test_ring_tier_duration() {
        assert_eq!(RingTier::SR.duration_days(), 7);
        assert_eq!(RingTier::MR.duration_days(), 30);
        assert_eq!(RingTier::LR.duration_days(), 90);
    }

    #[test]
    fn test_algorithm_entry_hash_format() {
        let valid_entry = AlgorithmEntry::new(
            "path/to/train_gpt.py".to_string(),
            [0u8; 32],
            vec![],
        );
        assert!(valid_entry.validate_hash().is_ok());

        // Verify hash is 32 bytes (can't construct invalid one due to type system)
        assert_eq!(valid_entry.entry_hash.len(), 32);
    }

    #[test]
    fn test_gate_activation() {
        let mut gate = Gate::new("test-gate".to_string(), GateType::Lf3BpbConvergence);
        assert!(gate.active);
        gate.deactivate();
        assert!(!gate.active);
        gate.activate();
        assert!(gate.active);
    }
}
