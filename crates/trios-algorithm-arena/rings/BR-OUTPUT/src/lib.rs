//! BR-OUTPUT — `AlgorithmArena` assembler ring.
//!
//! Bronze-tier ring under `trios-algorithm-arena` GOLD II crate. This is
//! the ring that downstream tooling actually calls into:
//!
//! - `AlgorithmArena::register(spec)` — verify the entry hash, store
//!   the manifest, return its `AlgorithmId`.
//! - `AlgorithmArena::run_one(algo, seed)` — invoke the registered
//!   trainer for one Fibonacci seed, capture a `BpbRow` and return its
//!   id. **Trainer is mocked** until SR-02 `TrainerRunner` lands; this
//!   ring is honest about that — see `RunBackend::Mocked`.
//! - `AlgorithmArena::list()` — list registered `(AlgorithmId, name)`.
//!
//! ## Honest disclosure (R5)
//!
//! The trainer invocation is gated through a `TrainerBackend` trait.
//! Until SR-02 ships its concrete `subprocess::PythonRunner`, the only
//! backend wired here is `MockedTrainer` — a deterministic stub that
//! returns a synthetic `BpbRow` keyed by `(algo_id, seed)`. No GPU
//! claims, no `val_bpb < 1.07063` claims, no Python spawn from a Bronze
//! ring. R-L6-PURE-007: `entry_path` is verified but never executed
//! here.
//!
//! ## Constitutional compliance
//!
//! - **R-RING-FACADE-001** — outer `src/lib.rs` only re-exports.
//! - **R-RING-DEP-002** — Bronze-tier deps only (`serde`, `serde_json`,
//!   `uuid`, `hex`, `sha2`, `thiserror`, plus the two SR-ALG sibling
//!   rings). NO tokio, NO subprocess, NO HTTP — those belong to BR-IO
//!   when SR-02 lands.
//! - **R-L6-PURE-007** — verifies entry hashes, never spawns Python.
//! - **L13** — single-ring scope.
//!
//! Closes #460 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//! Soul: Arena-Anchor

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use trios_algorithm_arena_sr_alg_00::{AlgorithmId, AlgorithmSpec};

// ───────────── public types ─────────────

/// Identifier of one captured BPB row, returned by `run_one`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BpbRowId(pub Uuid);

impl BpbRowId {
    /// Generate a fresh v4 id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BpbRowId {
    fn default() -> Self {
        Self::new()
    }
}

/// One captured BPB measurement.
///
/// In production this row is mirrored into `bpb_samples` (Neon) by the
/// trainer-runner. Here we only carry the in-process record so the
/// arena can return its id and downstream callers can fetch the full
/// row from whichever sink they prefer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BpbRow {
    /// This row's id.
    pub id: BpbRowId,
    /// Algorithm under measurement.
    pub algo_id: AlgorithmId,
    /// Random seed used (Fibonacci member for IGLA submissions).
    pub seed: i64,
    /// Measured `val_bpb` (mocked trainer returns a deterministic
    /// synthetic value).
    pub val_bpb: f64,
    /// Whether this row is from a real GPU run or a mocked trainer.
    pub backend: RunBackend,
}

/// Where the BPB row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunBackend {
    /// Real trainer (SR-02 + GPU). Not yet wired.
    Real,
    /// Deterministic mock — used until SR-02 ships.
    Mocked,
}

/// Errors surfaced by the arena.
#[derive(Debug, Error)]
pub enum ArenaError {
    /// Entry hash on disk did not match the manifest. The actual hash
    /// is reported for triage but is never trusted as a replacement.
    #[error("entry hash mismatch for {name}: expected {expected}, got {actual}")]
    EntryHashMismatch {
        /// Algorithm name.
        name: String,
        /// Hex of the expected hash from the manifest.
        expected: String,
        /// Hex of the hash actually computed from `entry_bytes`.
        actual: String,
    },
    /// Algorithm id was not registered.
    #[error("unknown algorithm id: {0}")]
    UnknownAlgorithm(AlgorithmId),
    /// Seed rejected by the trainer backend (e.g. not on the Fibonacci
    /// allow-list when strict mode is enabled).
    #[error("seed {0} rejected by trainer backend")]
    SeedRejected(i64),
}

// ───────────── trainer backend trait ─────────────

/// Pluggable trainer backend.
///
/// A concrete implementation will land in BR-IO once SR-02 ships its
/// `PythonRunner`. Until then `MockedTrainer` is the only honest
/// backend.
pub trait TrainerBackend: Send + Sync {
    /// Run one seed against an `AlgorithmSpec`. Returns `(val_bpb,
    /// backend)`. The backend tag is propagated into the `BpbRow` for
    /// honest provenance.
    fn run(&self, spec: &AlgorithmSpec, seed: i64) -> Result<(f64, RunBackend), ArenaError>;
}

/// Deterministic mock — returns a synthetic `val_bpb` keyed by
/// `(algo_id, seed)`. Useful for the assembler's own tests and for any
/// downstream pipeline that wants to exercise `run_one` plumbing
/// without burning GPU minutes.
#[derive(Debug, Default, Clone)]
pub struct MockedTrainer;

impl TrainerBackend for MockedTrainer {
    fn run(&self, spec: &AlgorithmSpec, seed: i64) -> Result<(f64, RunBackend), ArenaError> {
        // Deterministic: derive a [0.0, 1.0) fraction from a SHA-256 of
        // the spec id concatenated with the seed bytes, then offset
        // around the architectural floor 2.19 so the mock never claims
        // to beat the target.
        let mut h = Sha256::new();
        h.update(spec.algorithm_id.as_uuid().as_bytes());
        h.update(seed.to_le_bytes());
        let bytes = h.finalize();
        let frac = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            / u32::MAX as f64;
        let val_bpb = 2.19 + frac * 0.10;
        Ok((val_bpb, RunBackend::Mocked))
    }
}

// ───────────── AlgorithmArena ─────────────

/// In-process registry that owns specs and dispatches `run_one`.
pub struct AlgorithmArena {
    inner: Mutex<ArenaState>,
    backend: Box<dyn TrainerBackend>,
}

#[derive(Default)]
struct ArenaState {
    specs: HashMap<AlgorithmId, AlgorithmSpec>,
}

impl AlgorithmArena {
    /// Build a new arena with the supplied trainer backend.
    pub fn new(backend: Box<dyn TrainerBackend>) -> Self {
        Self {
            inner: Mutex::new(ArenaState::default()),
            backend,
        }
    }

    /// Build an arena pre-wired with the [`MockedTrainer`] backend.
    pub fn with_mock() -> Self {
        Self::new(Box::new(MockedTrainer))
    }

    /// Register a spec. The caller MUST supply the actual bytes of the
    /// entry script so we can hash-verify against `spec.entry_hash`
    /// before storing. R-L6-PURE-007: we never read the path ourselves.
    pub fn register(
        &self,
        spec: AlgorithmSpec,
        entry_bytes: &[u8],
    ) -> Result<AlgorithmId, ArenaError> {
        let actual = {
            let mut h = Sha256::new();
            h.update(entry_bytes);
            let out = h.finalize();
            let mut buf = [0u8; 32];
            buf.copy_from_slice(&out);
            buf
        };
        if !spec.verify_hash(&actual) {
            return Err(ArenaError::EntryHashMismatch {
                name: spec.name.clone(),
                expected: spec.entry_hash.to_string(),
                actual: hex::encode(actual),
            });
        }
        let id = spec.algorithm_id;
        let mut g = self.inner.lock().expect("arena poisoned");
        g.specs.insert(id, spec);
        Ok(id)
    }

    /// Look up a registered spec by id.
    pub fn get(&self, id: AlgorithmId) -> Option<AlgorithmSpec> {
        let g = self.inner.lock().expect("arena poisoned");
        g.specs.get(&id).cloned()
    }

    /// List registered `(id, name)` pairs in stable order.
    pub fn list(&self) -> Vec<(AlgorithmId, String)> {
        let g = self.inner.lock().expect("arena poisoned");
        let mut v: Vec<_> = g
            .specs
            .iter()
            .map(|(id, s)| (*id, s.name.clone()))
            .collect();
        v.sort_by(|a, b| a.1.cmp(&b.1));
        v
    }

    /// Run one seed against a registered algorithm. Returns the
    /// captured `BpbRow` (caller decides where to mirror it: `bpb_samples`
    /// over Neon, JSONL on disk, etc.).
    pub fn run_one(&self, algo: AlgorithmId, seed: i64) -> Result<BpbRow, ArenaError> {
        let spec = self
            .get(algo)
            .ok_or(ArenaError::UnknownAlgorithm(algo))?;
        let (val_bpb, backend) = self.backend.run(&spec, seed)?;
        Ok(BpbRow {
            id: BpbRowId::new(),
            algo_id: algo,
            seed,
            val_bpb,
            backend,
        })
    }
}

// ───────────── tests ─────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;
    use trios_algorithm_arena_sr_alg_00::{AlgorithmSpec, EntryHash};
    use trios_algorithm_arena_sr_alg_03::{build_spec, E2eTttEnv, ENTRY_PATH, FIBONACCI_SEEDS};

    fn hash_of(bytes: &[u8]) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(bytes);
        let out = h.finalize();
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&out);
        buf
    }

    fn fixture_spec(bytes: &[u8]) -> AlgorithmSpec {
        let env = E2eTttEnv::default();
        build_spec(EntryHash(hash_of(bytes)), &env)
    }

    #[test]
    fn register_accepts_matching_hash() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# stub e2e-ttt entry";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec, bytes).expect("hash matches");
        assert!(arena.get(id).is_some());
    }

    #[test]
    fn register_rejects_hash_mismatch() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# real entry";
        let spec = fixture_spec(bytes);
        let err = arena.register(spec, b"# tampered").unwrap_err();
        assert!(matches!(err, ArenaError::EntryHashMismatch { .. }));
    }

    #[test]
    fn list_is_stable_sorted_by_name() {
        let arena = AlgorithmArena::with_mock();
        // build two specs with distinct names
        let bytes_a = b"# a";
        let mut spec_a = fixture_spec(bytes_a);
        spec_a.name = "alpha".into();
        spec_a.entry_path = PathBuf::from(ENTRY_PATH);
        let bytes_b = b"# b";
        let mut spec_b = fixture_spec(bytes_b);
        spec_b.name = "beta".into();
        spec_b.entry_path = PathBuf::from(ENTRY_PATH);
        arena.register(spec_b, bytes_b).unwrap();
        arena.register(spec_a, bytes_a).unwrap();
        let listed: Vec<_> = arena.list().into_iter().map(|(_, n)| n).collect();
        assert_eq!(listed, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn run_one_returns_bpb_row_for_registered_algo() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# stub e2e-ttt entry";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec, bytes).unwrap();
        let row = arena.run_one(id, FIBONACCI_SEEDS[0]).unwrap();
        assert_eq!(row.algo_id, id);
        assert_eq!(row.seed, FIBONACCI_SEEDS[0]);
        assert_eq!(row.backend, RunBackend::Mocked);
        // Mocked floor: never claims to beat the architectural floor.
        assert!(row.val_bpb >= 2.19);
        assert!(row.val_bpb < 2.30);
    }

    #[test]
    fn run_one_unknown_algo_errors() {
        let arena = AlgorithmArena::with_mock();
        let err = arena.run_one(AlgorithmId::new(), 1597).unwrap_err();
        assert!(matches!(err, ArenaError::UnknownAlgorithm(_)));
    }

    #[test]
    fn mocked_backend_is_deterministic_per_seed() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# determinism check";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec, bytes).unwrap();
        let r1 = arena.run_one(id, 2584).unwrap();
        let r2 = arena.run_one(id, 2584).unwrap();
        assert!((r1.val_bpb - r2.val_bpb).abs() < 1e-12);
    }

    #[test]
    fn mocked_backend_varies_with_seed() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# variance check";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec, bytes).unwrap();
        let r1 = arena.run_one(id, FIBONACCI_SEEDS[0]).unwrap();
        let r2 = arena.run_one(id, FIBONACCI_SEEDS[1]).unwrap();
        assert!((r1.val_bpb - r2.val_bpb).abs() > 1e-9);
    }

    #[test]
    fn integration_e2e_ttt_register_then_run_three_seeds() {
        // The headline integration test the issue calls out: register
        // the e2e-ttt spec from SR-ALG-03 and run all three Fibonacci
        // seeds. Trainer is mocked — this test verifies the assembler
        // wiring end-to-end, not the GPU sweep.
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# pretend train_gpt.py";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec, bytes).unwrap();
        let mut rows = Vec::new();
        for seed in FIBONACCI_SEEDS {
            let row = arena.run_one(id, seed).unwrap();
            assert_eq!(row.algo_id, id);
            assert_eq!(row.backend, RunBackend::Mocked);
            rows.push(row);
        }
        // Three distinct row ids.
        assert_eq!(rows.len(), 3);
        let ids: std::collections::HashSet<_> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn entry_hash_mismatch_reports_both_hashes() {
        let arena = AlgorithmArena::with_mock();
        let real = b"real bytes";
        let spec = fixture_spec(real);
        let err = arena.register(spec, b"different bytes").unwrap_err();
        match err {
            ArenaError::EntryHashMismatch {
                expected, actual, ..
            } => {
                assert_eq!(expected.len(), 64);
                assert_eq!(actual.len(), 64);
                assert_ne!(expected, actual);
            }
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn run_one_propagates_backend_tag() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# tag check";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec, bytes).unwrap();
        let row = arena.run_one(id, 1597).unwrap();
        // R5-honest: mocked backend must never claim to be Real.
        assert_ne!(row.backend, RunBackend::Real);
    }

    #[test]
    fn arena_get_returns_clone() {
        let arena = AlgorithmArena::with_mock();
        let bytes = b"# clone check";
        let spec = fixture_spec(bytes);
        let id = arena.register(spec.clone(), bytes).unwrap();
        let got = arena.get(id).expect("registered");
        assert_eq!(got.name, spec.name);
        assert_eq!(got.entry_hash, spec.entry_hash);
    }
}
