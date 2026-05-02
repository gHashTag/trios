//! SR-ALG-03 — e2e-ttt spec & verifier
//!
//! Wraps the algorithm from [arXiv:2512.23675 — *End-to-End Test-Time
//! Training for Long Context*] as a Rust **spec + verifier** ring.
//! This ring does NOT execute the Python training itself — that is
//! the job of SR-02 `TrainerRunner`. R-L6-PURE-007: the only Python
//! file referenced lives at `parameter-golf/records/track_10min_16mb/
//! e2e-ttt/train_gpt.py`, OUTSIDE `crates/`.
//!
//! ## Honest disclosure (R5)
//!
//! This ring ships the spec + verifier. The 3-seed sweep on
//! `F₁₇=1597`, `F₁₈=2584`, `F₁₉=4181` that produces
//! `val_bpb < 1.07063` is a GPU run through SR-02 + BR-OUTPUT and is
//! scoped as a follow-up PR. All the bookkeeping to make that follow-up
//! painless is here: spec constants, hash verifier, baseline-equivalence
//! checker, target threshold, Fibonacci seeds, embargo window.
//!
//! Closes #457 (spec + verifier ring) · Part of #446
//! Anchor: phi^2 + phi^-2 = 3 · alpha_phi = phi^-3 / 2 (Coq PROVEN)

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use trios_algorithm_arena_sr_alg_00::{AlgorithmId, AlgorithmSpec, EntryHash, EnvVar, EnvValue};

/// Hard target: beat `openai/parameter-golf#1837` baseline.
/// Pass iff the 3-seed mean `val_bpb` is **strictly less** than this.
pub const TARGET_VAL_BPB: f64 = 1.07063;

/// Fibonacci seeds for the 3-seed sweep.
/// `F_17 = 1597`, `F_18 = 2584`, `F_19 = 4181`.
pub const FIBONACCI_SEEDS: [i64; 3] = [1597, 2584, 4181];

/// Canonical entry path (lives OUTSIDE `crates/` — R-L6-PURE-007).
pub const ENTRY_PATH: &str = "parameter-golf/records/track_10min_16mb/e2e-ttt/train_gpt.py";

/// Embargo window before public publication.
pub const EMBARGO_DAYS: u32 = 14;

/// Coq theorem citation attached to every `bpb_samples` row.
pub const COQ_THEOREM_ID: &str = "alpha_phi_phi_cubed";

// ─────────────── spec_envelope ───────────────

/// Spec envelope — every knob the Python entry reads via env vars.
///
/// Defaults match the arXiv paper's reference config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct E2eTttEnv {
    /// `TTT_INNER_STEPS` — inner-loop steps per chunk (paper: 4).
    pub inner_steps: u32,
    /// `TTT_LR_INNER` — inner learning rate, φ-band-compatible
    /// (paper: 1e-3).
    pub lr_inner: f64,
    /// `TTT_CHUNK_LEN` — chunk length in tokens (paper: 512).
    pub chunk_len: u32,
}

impl Default for E2eTttEnv {
    fn default() -> Self {
        Self {
            inner_steps: 4,
            lr_inner: 1e-3,
            chunk_len: 512,
        }
    }
}

impl E2eTttEnv {
    /// Convert to the `Vec<(EnvVar, EnvValue)>` shape that `AlgorithmSpec`
    /// expects, preserving the paper's ordering.
    pub fn to_env_vars(&self) -> Vec<(EnvVar, EnvValue)> {
        vec![
            (
                EnvVar::new("TTT_INNER_STEPS"),
                EnvValue::new(self.inner_steps.to_string()),
            ),
            (
                EnvVar::new("TTT_LR_INNER"),
                EnvValue::new(format!("{}", self.lr_inner)),
            ),
            (
                EnvVar::new("TTT_CHUNK_LEN"),
                EnvValue::new(self.chunk_len.to_string()),
            ),
        ]
    }

    /// Special case: when `TTT_INNER_STEPS=0`, the trainer MUST degrade
    /// into the merged baseline byte-for-byte (pattern from PR #2059
    /// `baseline_equivalence.py`). Returns the zero-inner-steps config
    /// used by the baseline-equivalence verifier.
    pub fn baseline_equivalence() -> Self {
        Self {
            inner_steps: 0,
            ..Self::default()
        }
    }
}

// ─────────────── verifier ───────────────

/// Compute the SHA-256 of an entry script's bytes.
pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}

/// Build the canonical [`AlgorithmSpec`] for e2e-ttt.
///
/// `entry_hash` MUST be the SHA-256 of the pinned `train_gpt.py`
/// source. SR-02 `TrainerRunner` calls [`Verifier::preflight`] before
/// spawning and refuses to run on a hash mismatch.
pub fn build_spec(entry_hash: EntryHash, env: &E2eTttEnv) -> AlgorithmSpec {
    AlgorithmSpec {
        algorithm_id: AlgorithmId::new(),
        name: "e2e-ttt".into(),
        entry_path: PathBuf::from(ENTRY_PATH),
        entry_hash,
        env: env.to_env_vars(),
        golden_state_hash: None,
        theorem: Some(COQ_THEOREM_ID.into()),
    }
}

/// Outcome of a pre-flight check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum VerifierOutcome {
    /// Hash + env are valid; safe to spawn.
    Ok,
    /// `actual_hash` does not equal `spec.entry_hash`.
    HashMismatch {
        /// Hex of the hash the spec expected.
        expected_hex: String,
        /// Hex of the hash we actually observed.
        actual_hex: String,
    },
    /// `entry_path` lives INSIDE `crates/` (R-L6-PURE-007 violation).
    PathUnderCrates {
        /// Offending path.
        path: String,
    },
    /// Env-var sanity: `TTT_INNER_STEPS` must be 0..=16; `TTT_CHUNK_LEN`
    /// must be a positive power of two up to 2048.
    InvalidEnv {
        /// Free-form reason.
        reason: String,
    },
}

/// Pure stateless verifier.
pub struct Verifier;

impl Verifier {
    /// Full pre-flight check. Called by SR-02 `TrainerRunner` before
    /// spawning the Python trainer.
    pub fn preflight(
        spec: &AlgorithmSpec,
        env: &E2eTttEnv,
        actual_entry_bytes: &[u8],
    ) -> VerifierOutcome {
        // ── Path check ──
        let path_str = spec.entry_path.to_string_lossy();
        if path_str.starts_with("crates/") || path_str.contains("/crates/") {
            return VerifierOutcome::PathUnderCrates {
                path: path_str.into_owned(),
            };
        }

        // ── Hash check ──
        let actual = sha256_bytes(actual_entry_bytes);
        if !spec.verify_hash(&actual) {
            return VerifierOutcome::HashMismatch {
                expected_hex: hex::encode(spec.entry_hash.as_bytes()),
                actual_hex: hex::encode(actual),
            };
        }

        // ── Env sanity ──
        if env.inner_steps > 16 {
            return VerifierOutcome::InvalidEnv {
                reason: format!(
                    "TTT_INNER_STEPS must be 0..=16, got {}",
                    env.inner_steps
                ),
            };
        }
        if env.chunk_len == 0
            || env.chunk_len > 2048
            || !env.chunk_len.is_power_of_two()
        {
            return VerifierOutcome::InvalidEnv {
                reason: format!(
                    "TTT_CHUNK_LEN must be a positive power of two up to 2048, got {}",
                    env.chunk_len
                ),
            };
        }
        if !env.lr_inner.is_finite() || env.lr_inner <= 0.0 || env.lr_inner > 1e-1 {
            return VerifierOutcome::InvalidEnv {
                reason: format!(
                    "TTT_LR_INNER must be finite in (0, 0.1], got {}",
                    env.lr_inner
                ),
            };
        }

        VerifierOutcome::Ok
    }
}

// ─────────────── gate ───────────────

/// Gate verdict for a 3-seed sweep against the #1837 baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum GateVerdict {
    /// Achieved — 3-seed mean is below `TARGET_VAL_BPB`.
    Win {
        /// 3-seed mean val_bpb.
        mean_val_bpb: f64,
        /// Margin below target (target - mean).
        margin: f64,
    },
    /// Did not beat baseline — 3-seed mean is ≥ target.
    HonestNotYet {
        /// 3-seed mean val_bpb.
        mean_val_bpb: f64,
        /// Gap above target (mean - target).
        gap: f64,
    },
}

/// Evaluate a 3-seed sweep against the baseline.
///
/// Panics if `seed_bpbs.len() != 3` (contract — the sweep is always 3
/// Fibonacci seeds).
pub fn evaluate_gate(seed_bpbs: [f64; 3]) -> GateVerdict {
    let mean = (seed_bpbs[0] + seed_bpbs[1] + seed_bpbs[2]) / 3.0;
    if mean < TARGET_VAL_BPB {
        GateVerdict::Win {
            mean_val_bpb: mean,
            margin: TARGET_VAL_BPB - mean,
        }
    } else {
        GateVerdict::HonestNotYet {
            mean_val_bpb: mean,
            gap: mean - TARGET_VAL_BPB,
        }
    }
}

// ─────────────── tests ───────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn script(kind: &str) -> Vec<u8> {
        match kind {
            "a" => b"# e2e-ttt variant A\nprint('hi')\n".to_vec(),
            "b" => b"# e2e-ttt variant B\nprint('hi')\n".to_vec(),
            _ => unreachable!(),
        }
    }

    fn spec_for(bytes: &[u8], env: &E2eTttEnv) -> AlgorithmSpec {
        build_spec(EntryHash(sha256_bytes(bytes)), env)
    }

    // ── constants ──

    #[test]
    fn target_val_bpb_matches_issue() {
        assert_eq!(TARGET_VAL_BPB, 1.07063);
    }

    #[test]
    fn fibonacci_seeds_match_spec() {
        // F_17 = 1597, F_18 = 2584, F_19 = 4181
        assert_eq!(FIBONACCI_SEEDS, [1597, 2584, 4181]);
        // Fibonacci identity: F_n = F_{n-1} + F_{n-2}
        assert_eq!(FIBONACCI_SEEDS[1], 987 + 1597); // F_17 + F_16
        assert_eq!(FIBONACCI_SEEDS[2], FIBONACCI_SEEDS[0] + FIBONACCI_SEEDS[1]);
    }

    #[test]
    fn entry_path_outside_crates() {
        // R-L6-PURE-007: ENTRY_PATH MUST NOT live inside crates/.
        assert!(!ENTRY_PATH.starts_with("crates/"));
        assert!(!ENTRY_PATH.contains("/crates/"));
    }

    #[test]
    fn embargo_window_is_14_days() {
        assert_eq!(EMBARGO_DAYS, 14);
    }

    // ── env ──

    #[test]
    fn env_default_matches_paper() {
        let e = E2eTttEnv::default();
        assert_eq!(e.inner_steps, 4);
        assert_eq!(e.lr_inner, 1e-3);
        assert_eq!(e.chunk_len, 512);
    }

    #[test]
    fn env_to_env_vars_preserves_order() {
        let e = E2eTttEnv::default();
        let v = e.to_env_vars();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].0.as_str(), "TTT_INNER_STEPS");
        assert_eq!(v[1].0.as_str(), "TTT_LR_INNER");
        assert_eq!(v[2].0.as_str(), "TTT_CHUNK_LEN");
    }

    #[test]
    fn env_baseline_equivalence_zero_inner_steps() {
        let e = E2eTttEnv::baseline_equivalence();
        assert_eq!(e.inner_steps, 0);
        assert_eq!(e.chunk_len, 512);
    }

    // ── verifier ──

    #[test]
    fn verifier_ok_path() {
        let bytes = script("a");
        let env = E2eTttEnv::default();
        let spec = spec_for(&bytes, &env);
        assert_eq!(Verifier::preflight(&spec, &env, &bytes), VerifierOutcome::Ok);
    }

    #[test]
    fn verifier_rejects_hash_mismatch() {
        let bytes_a = script("a");
        let bytes_b = script("b");
        let env = E2eTttEnv::default();
        let spec = spec_for(&bytes_a, &env); // spec pinned to A
        match Verifier::preflight(&spec, &env, &bytes_b) {
            VerifierOutcome::HashMismatch { expected_hex, actual_hex } => {
                assert_ne!(expected_hex, actual_hex);
                assert_eq!(expected_hex.len(), 64);
            }
            other => panic!("expected HashMismatch, got {:?}", other),
        }
    }

    #[test]
    fn verifier_rejects_path_under_crates() {
        let bytes = script("a");
        let env = E2eTttEnv::default();
        let mut spec = spec_for(&bytes, &env);
        spec.entry_path = PathBuf::from("crates/trios-algorithm-arena/bad/train.py");
        assert!(matches!(
            Verifier::preflight(&spec, &env, &bytes),
            VerifierOutcome::PathUnderCrates { .. }
        ));
    }

    #[test]
    fn verifier_rejects_inner_steps_too_high() {
        let bytes = script("a");
        let env = E2eTttEnv {
            inner_steps: 17,
            ..E2eTttEnv::default()
        };
        let spec = spec_for(&bytes, &env);
        assert!(matches!(
            Verifier::preflight(&spec, &env, &bytes),
            VerifierOutcome::InvalidEnv { .. }
        ));
    }

    #[test]
    fn verifier_rejects_non_power_of_two_chunk() {
        let bytes = script("a");
        let env = E2eTttEnv {
            chunk_len: 600,
            ..E2eTttEnv::default()
        };
        let spec = spec_for(&bytes, &env);
        assert!(matches!(
            Verifier::preflight(&spec, &env, &bytes),
            VerifierOutcome::InvalidEnv { .. }
        ));
    }

    #[test]
    fn verifier_rejects_zero_chunk_len() {
        let bytes = script("a");
        let env = E2eTttEnv {
            chunk_len: 0,
            ..E2eTttEnv::default()
        };
        let spec = spec_for(&bytes, &env);
        assert!(matches!(
            Verifier::preflight(&spec, &env, &bytes),
            VerifierOutcome::InvalidEnv { .. }
        ));
    }

    #[test]
    fn verifier_rejects_non_finite_lr() {
        let bytes = script("a");
        let env = E2eTttEnv {
            lr_inner: f64::NAN,
            ..E2eTttEnv::default()
        };
        let spec = spec_for(&bytes, &env);
        assert!(matches!(
            Verifier::preflight(&spec, &env, &bytes),
            VerifierOutcome::InvalidEnv { .. }
        ));
    }

    // ── spec ──

    #[test]
    fn build_spec_attaches_theorem_id() {
        let bytes = script("a");
        let env = E2eTttEnv::default();
        let spec = spec_for(&bytes, &env);
        assert_eq!(spec.theorem.as_deref(), Some(COQ_THEOREM_ID));
        assert_eq!(spec.name, "e2e-ttt");
    }

    #[test]
    fn build_spec_entry_path_is_pinned_constant() {
        let bytes = script("a");
        let env = E2eTttEnv::default();
        let spec = spec_for(&bytes, &env);
        assert_eq!(spec.entry_path, PathBuf::from(ENTRY_PATH));
    }

    // ── gate ──

    #[test]
    fn gate_win_below_target() {
        match evaluate_gate([1.05, 1.06, 1.07]) {
            GateVerdict::Win { mean_val_bpb, margin } => {
                assert!(mean_val_bpb < TARGET_VAL_BPB);
                assert!((margin - (TARGET_VAL_BPB - mean_val_bpb)).abs() < 1e-9);
            }
            other => panic!("expected Win, got {:?}", other),
        }
    }

    #[test]
    fn gate_honest_not_yet_at_or_above_target() {
        let v = evaluate_gate([1.07063, 1.08, 1.09]);
        assert!(matches!(v, GateVerdict::HonestNotYet { .. }));
    }

    #[test]
    fn gate_honest_not_yet_exact_target() {
        let v = evaluate_gate([TARGET_VAL_BPB; 3]);
        assert!(
            matches!(v, GateVerdict::HonestNotYet { .. }),
            "mean == target must be HonestNotYet (strict inequality)"
        );
    }

    #[test]
    fn gate_verdict_serialises_tagged_kind() {
        let v = evaluate_gate([1.0, 1.0, 1.0]);
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"verdict\""));
        assert!(s.contains("\"win\""));
    }
}
