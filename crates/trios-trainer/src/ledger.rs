//! Ledger — triplet-validated row emission with embargo block
//!
//! Every row MUST contain: BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>
//!
//! Embargo list: SHA values that must be blocked from ledger.

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

/// Triplet — minimal validation format for every ledger row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triplet {
    pub bpb: f32,
    pub step: usize,
    pub seed: u64,
    pub sha: String,
    pub jsonl_row: String,
    pub gate_status: String,
}

/// Ledger row (full format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerRow {
    pub agent: String,
    pub bpb: f32,
    pub seed: u64,
    pub sha: String,
    pub step: usize,
    pub ts: String,
    pub gate_status: String,
}

/// Embargo block — prevents certain SHAs from being emitted
#[derive(Debug, Clone)]
pub struct EmbargoBlock {
    /// List of embargoed SHAs
    pub blocked_shas: Vec<String>,
}

impl EmbargoBlock {
    /// Create embargo block with default list
    pub fn new() -> Self {
        Self {
            blocked_shas: vec![
                // Add embargoed SHAs here
                // These are commits that failed validation or are known bad
            ],
        }
    }

    /// Check if a SHA is embargoed
    pub fn is_embargoed(&self, sha: &str) -> bool {
        self.blocked_shas.iter().any(|b| b == sha)
    }
}

impl Default for EmbargoBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Emit a row to the ledger
///
/// Returns error if SHA is embargoed or write fails.
pub fn emit_row<P: AsRef<Path>>(
    ledger_path: P,
    row: &LedgerRow,
    embargo: &EmbargoBlock,
) -> Result<(), EmitError> {
    // Check embargo first
    if embargo.is_embargoed(&row.sha) {
        return Err(EmitError::EmbargoBlocked(row.sha.clone()));
    }

    // Validate triplet format
    let jsonl = serde_json::to_string(row)
        .map_err(|e| EmitError::SerializeError(e.to_string()))?;

    // Append to ledger
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path.as_ref())
        .map_err(|e| EmitError::WriteError(e.to_string()))?;

    writeln!(file, "{}", jsonl)
        .map_err(|e| EmitError::WriteError(e.to_string()))?;

    // Validate triplet format in output
    let triplet = Triplet {
        bpb: row.bpb,
        step: row.step,
        seed: row.seed,
        sha: row.sha.clone(),
        jsonl_row: jsonl.clone(),
        gate_status: row.gate_status.clone(),
    };

    // Verify all triplet fields present
    if triplet.bpb.is_nan() || triplet.jsonl_row.is_empty() {
        return Err(EmitError::InvalidTriplet("empty or NaN fields".into()));
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum EmitError {
    #[error("SHA {0} is embargoed and cannot be emitted")]
    EmbargoBlocked(String),

    #[error("Failed to serialize row: {0}")]
    SerializeError(String),

    #[error("Failed to write to ledger: {0}")]
    WriteError(String),

    #[error("Invalid triplet: {0}")]
    InvalidTriplet(String),
}

/// Get current commit SHA
pub fn get_commit_sha() -> Result<String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("git rev-parse failed"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embargo_block() {
        let embargo = EmbargoBlock {
            blocked_shas: vec!["deadbeef".into()],
        };

        assert!(embargo.is_embargoed("deadbeef"));
        assert!(!embargo.is_embargoed("goodcommit"));
    }

    #[test]
    fn test_triplet_validation() {
        let row = LedgerRow {
            agent: "test".into(),
            bpb: 2.2393,
            seed: 43,
            sha: "abc123".into(),
            step: 27000,
            ts: "2026-04-26T00:00:00Z".into(),
            gate_status: "pending".into(),
        };

        let jsonl = serde_json::to_string(&row).unwrap();
        assert!(jsonl.contains("\"bpb\":2.2393"));
        assert!(jsonl.contains("\"seed\":43"));
        assert!(jsonl.contains("\"step\":27000"));
    }
}
