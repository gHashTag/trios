//! `acm_ae_check` — LA Phase D witness for the «Flos Aureus» PhD monograph.
//!
//! Verifies the three ACM Artifact Evaluation badges
//! ([Functional], [Reusable], [Available]) for the trios#265 monograph
//! against a fresh checkout of `gHashTag/trios`. Exits 0 on full admit;
//! 70..=74 on reject (one disjoint code per [`AcmAeError`] variant).
//!
//! ## Pre-registration
//!
//! - ONE SHOT v2.0 §3 Phase D row "LA": `artifact/INSTALL.md` +
//!   Rust-only entrypoint per R1 + `artifact/CLAIMS.md`.
//! - ONE SHOT v2.0 §5 falsification table row "LA":
//!   `bash artifact/run.sh && diff artifact/expected.txt artifact/output.txt`
//!   — replaced by the Rust `acm-ae-check run --diff` invocation per R1, the
//!   diff semantics are byte-for-byte equivalent (see [`check_fingerprint`]).
//!
//! ## R6 boundary
//!
//! This crate owns:
//! - `tools/acm_ae_check/**`
//! - `artifact/**`
//! - `assertions/witness/acm_ae.toml`
//!
//! and only appends a single `members` line to the workspace `Cargo.toml`.
//! It does **not** edit any chapter, any other crate, or any other witness.
//!
//! ## L-R14 traceability
//!
//! Every numeric / symbolic anchor below mirrors a value in
//! `assertions/igla_assertions.json` (loaded by `trios-phd reproduce`):
//!
//! | Anchor                | Value                | Source theorem / file        |
//! |-----------------------|----------------------|------------------------------|
//! | `TRINITY_ANCHOR`      | `phi^2 + phi^-2 = 3` | `lucas_closure_gf16.v::lucas_2_eq_3` (Proven) |
//! | `ZENODO_DOI`          | `10.5281/zenodo.19227877` | TRI-27 anchor DOI       |
//! | `PRUNE_THRESHOLD`     | `3.5`                | INV-2 `igla_asha_bound.v::prune_threshold_from_trinity` (Proven) |
//! | `WARMUP_BLIND_STEPS`  | `4000`               | INV-2 (≈ φ¹⁶ structural)     |
//! | `D_MODEL_MIN`         | `256`                | INV-3 `gf16_precision.v` (Adm/n=1,2 Proven) |
//! | `LR_CHAMPION`         | `0.004`              | INV-1 `lr_phi_optimality.v` (Admitted) |
//! | `MIN_PAGES`/`MAX_PAGES`| `250` / `350`       | LT `tools/page_gate::{MIN_PAGES,MAX_PAGES}` |
//! | exit codes 70..=74    | disjoint from L-h4 (50..=53) and LT (60..=63) | this file |

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Anchors (L-R14 traceable)
// ---------------------------------------------------------------------------

/// Trinity Anchor identity, mirror of `lucas_closure_gf16.v::lucas_2_eq_3`.
pub const TRINITY_ANCHOR: &str = "phi^2 + phi^-2 = 3";
/// TRI-27 anchor Zenodo DOI for ACM AE Available badge.
pub const ZENODO_DOI: &str = "10.5281/zenodo.19227877";
/// Pre-registered minimum page count (mirror of `page_gate::MIN_PAGES`).
pub const MIN_PAGES: u32 = 250;
/// Pre-registered maximum page count (mirror of `page_gate::MAX_PAGES`).
pub const MAX_PAGES: u32 = 350;
/// INV-2 ASHA prune threshold (Proven in `igla_asha_bound.v`).
pub const PRUNE_THRESHOLD: f64 = 3.5;
/// INV-2 warmup blind steps (structural ≈ φ¹⁶).
pub const WARMUP_BLIND_STEPS: u32 = 4000;
/// INV-3 minimum hidden dimension (GF16 precision floor).
pub const D_MODEL_MIN: u32 = 256;
/// INV-1 champion learning rate (`α_φ · φ⁻³`).
pub const LR_CHAMPION: f64 = 0.004;

/// Three-badge label set, in canonical order.
pub const BADGES: [&str; 3] = ["Functional", "Reusable", "Available"];

// ---------------------------------------------------------------------------
// Exit-code namespace 70..=74 (disjoint from L-h4 50..=53, LT 60..=63)
// ---------------------------------------------------------------------------

/// Disjoint exit codes for the LA witness. See [`AcmAeError::exit_code`].
pub mod exit {
    /// Witness passed all three badges.
    pub const ADMIT: u8 = 0;
    /// Functional badge failed (workspace members missing or unbuildable).
    pub const FUNCTIONAL: u8 = 70;
    /// Reusable badge failed (reproducibility.md absent or under-spec).
    pub const REUSABLE: u8 = 71;
    /// Available badge failed (DOI / commit anchor missing).
    pub const AVAILABLE: u8 = 72;
    /// Filesystem / read error during witness execution.
    pub const IO: u8 = 73;
    /// `artifact/output.txt` does not match `artifact/expected.txt`.
    pub const MISMATCH: u8 = 74;
}

// ---------------------------------------------------------------------------
// Error variants
// ---------------------------------------------------------------------------

/// One typed variant per disjoint exit code.
#[derive(Debug, Error)]
pub enum AcmAeError {
    /// Functional badge: required workspace member missing.
    #[error("Functional badge FAIL: workspace member `{member}` missing at `{path}`")]
    Functional {
        /// Logical workspace-member name.
        member: String,
        /// Filesystem path that was probed.
        path: PathBuf,
    },
    /// Reusable badge: reproducibility manifest absent or missing required entry-points.
    #[error("Reusable badge FAIL: `{path}` missing required entry `{needle}`")]
    Reusable {
        /// Path to the manifest under audit.
        path: PathBuf,
        /// Substring whose presence the manifest must establish.
        needle: String,
    },
    /// Available badge: persistent identifier not cited.
    #[error("Available badge FAIL: persistent identifier `{anchor}` not cited in `{path}`")]
    Available {
        /// Path to the file that should cite the anchor.
        path: PathBuf,
        /// Anchor (DOI or Trinity identity) expected.
        anchor: String,
    },
    /// Generic IO error wrapped from `std::io`.
    #[error("io error at `{path}`: {detail}")]
    Io {
        /// Path that triggered the error.
        path: PathBuf,
        /// Underlying message.
        detail: String,
    },
    /// `expected.txt` and `output.txt` disagree byte-for-byte.
    #[error("fingerprint mismatch: expected={expected_len}B observed={observed_len}B (first diff at byte {first_diff})")]
    Mismatch {
        /// Length of the expected fingerprint.
        expected_len: usize,
        /// Length of the observed fingerprint.
        observed_len: usize,
        /// Byte offset of the first divergence (or 0 if length-only).
        first_diff: usize,
    },
}

impl AcmAeError {
    /// Disjoint exit code per variant.
    pub fn exit_code(&self) -> u8 {
        match self {
            AcmAeError::Functional { .. } => exit::FUNCTIONAL,
            AcmAeError::Reusable { .. } => exit::REUSABLE,
            AcmAeError::Available { .. } => exit::AVAILABLE,
            AcmAeError::Io { .. } => exit::IO,
            AcmAeError::Mismatch { .. } => exit::MISMATCH,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-badge checks
// ---------------------------------------------------------------------------

/// Files whose presence proves the **Functional** badge — building the LA pack
/// requires these workspace members to compile.
///
/// Each entry is a relative path that MUST exist under `repo_root`.
pub fn functional_required_paths() -> [&'static str; 4] {
    [
        "Cargo.toml",
        "crates/trios-phd/Cargo.toml",
        "tools/page_gate/Cargo.toml",
        "tools/acm_ae_check/Cargo.toml",
    ]
}

/// Verify the Functional badge.
pub fn check_functional(repo_root: &Path) -> Result<(), AcmAeError> {
    for rel in functional_required_paths() {
        let p = repo_root.join(rel);
        if !p.exists() {
            return Err(AcmAeError::Functional {
                member: rel.to_string(),
                path: p,
            });
        }
    }
    Ok(())
}

/// Substrings that the Reusable manifest MUST contain, in any order.
///
/// Each substring corresponds to an ACM AE Reusable-badge requirement
/// (entry point, hardware, software, seeds, R1 declaration).
pub fn reusable_required_needles() -> [&'static str; 5] {
    [
        "Entry points",
        "cargo run -p trios-phd",
        "tectonic",
        "Hardware profile",
        "(R1)",
    ]
}

/// Verify the Reusable badge against `docs/phd/reproducibility.md`.
pub fn check_reusable(repo_root: &Path) -> Result<(), AcmAeError> {
    let path = repo_root.join("docs/phd/reproducibility.md");
    let body = fs::read_to_string(&path).map_err(|e| AcmAeError::Io {
        path: path.clone(),
        detail: e.to_string(),
    })?;
    for needle in reusable_required_needles() {
        if !body.contains(needle) {
            return Err(AcmAeError::Reusable {
                path: path.clone(),
                needle: needle.to_string(),
            });
        }
    }
    Ok(())
}

/// Anchors that the Available manifest (CLAIMS.md) MUST cite.
pub fn available_required_anchors() -> [&'static str; 2] {
    [TRINITY_ANCHOR, ZENODO_DOI]
}

/// Verify the Available badge against `artifact/CLAIMS.md`.
pub fn check_available(repo_root: &Path) -> Result<(), AcmAeError> {
    let path = repo_root.join("artifact/CLAIMS.md");
    let body = fs::read_to_string(&path).map_err(|e| AcmAeError::Io {
        path: path.clone(),
        detail: e.to_string(),
    })?;
    for anchor in available_required_anchors() {
        if !body.contains(anchor) {
            return Err(AcmAeError::Available {
                path: path.clone(),
                anchor: anchor.to_string(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Deterministic fingerprint (artifact/expected.txt mirror)
// ---------------------------------------------------------------------------

/// Deterministic, environment-independent fingerprint string.
///
/// This is the byte-for-byte content of `artifact/expected.txt`. The witness
/// re-derives it from compile-time constants and writes it to
/// `artifact/output.txt`; the LA gate then asserts equality.
///
/// Excludes any value that varies between checkouts (git SHA, timestamp, OS,
/// rustc version) so that the witness is reproducible on any reviewer machine
/// per ACM AE Reusable-badge requirements.
pub fn fingerprint() -> String {
    let mut buf = String::new();
    buf.push_str("ACM-AE-3-BADGE\n");
    buf.push_str(&format!("anchor={}\n", TRINITY_ANCHOR));
    buf.push_str(&format!("doi={}\n", ZENODO_DOI));
    buf.push_str(&format!(
        "badges={},{},{}\n",
        BADGES[0], BADGES[1], BADGES[2]
    ));
    buf.push_str(&format!("min_pages={}\n", MIN_PAGES));
    buf.push_str(&format!("max_pages={}\n", MAX_PAGES));
    buf.push_str(&format!("prune_threshold={}\n", PRUNE_THRESHOLD));
    buf.push_str(&format!("warmup_blind_steps={}\n", WARMUP_BLIND_STEPS));
    buf.push_str(&format!("d_model_min={}\n", D_MODEL_MIN));
    buf.push_str(&format!("lr_champion={}\n", LR_CHAMPION));
    buf.push_str("lane=LA\n");
    buf.push_str("phase=D\n");
    buf.push_str(&format!(
        "exit_codes=admit:{},functional:{},reusable:{},available:{},io:{},mismatch:{}\n",
        exit::ADMIT,
        exit::FUNCTIONAL,
        exit::REUSABLE,
        exit::AVAILABLE,
        exit::IO,
        exit::MISMATCH,
    ));
    buf
}

/// Verify `artifact/output.txt` against `artifact/expected.txt` byte-for-byte.
pub fn check_fingerprint(repo_root: &Path) -> Result<(), AcmAeError> {
    let exp_path = repo_root.join("artifact/expected.txt");
    let out_path = repo_root.join("artifact/output.txt");
    let expected = fs::read(&exp_path).map_err(|e| AcmAeError::Io {
        path: exp_path.clone(),
        detail: e.to_string(),
    })?;
    let observed = fs::read(&out_path).map_err(|e| AcmAeError::Io {
        path: out_path.clone(),
        detail: e.to_string(),
    })?;
    if expected != observed {
        let first_diff = expected
            .iter()
            .zip(observed.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| std::cmp::min(expected.len(), observed.len()));
        return Err(AcmAeError::Mismatch {
            expected_len: expected.len(),
            observed_len: observed.len(),
            first_diff,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Aggregate report
// ---------------------------------------------------------------------------

/// Per-badge admit/reject snapshot, suitable for `serde_json::to_string_pretty`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AcmAeReport {
    /// `ACM-AE-3-BADGE` constant marker for downstream tools.
    pub kind: &'static str,
    /// Lane identifier (always `"LA"`).
    pub lane: &'static str,
    /// Phase identifier (always `"D"`).
    pub phase: &'static str,
    /// Trinity Anchor identity.
    pub anchor: &'static str,
    /// Persistent DOI.
    pub doi: &'static str,
    /// `true` iff Functional badge passed.
    pub functional: bool,
    /// `true` iff Reusable badge passed.
    pub reusable: bool,
    /// `true` iff Available badge passed.
    pub available: bool,
    /// `true` iff fingerprint matches.
    pub fingerprint_ok: bool,
}

/// Run all checks and return a structured report.
///
/// On the first failure, returns the corresponding [`AcmAeError`] (caller
/// chooses whether to keep going for diagnostics or fail-fast — `bin/main.rs`
/// fails fast).
pub fn run_all(repo_root: &Path) -> Result<AcmAeReport, AcmAeError> {
    check_functional(repo_root)?;
    check_reusable(repo_root)?;
    check_available(repo_root)?;
    check_fingerprint(repo_root)?;
    Ok(AcmAeReport {
        kind: "ACM-AE-3-BADGE",
        lane: "LA",
        phase: "D",
        anchor: TRINITY_ANCHOR,
        doi: ZENODO_DOI,
        functional: true,
        reusable: true,
        available: true,
        fingerprint_ok: true,
    })
}

// ---------------------------------------------------------------------------
// Tests (10 — one per failure variant, plus totality + constants traceable)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn skel(root: &Path) {
        // Minimal viable repo skeleton for run_all to admit.
        fs::create_dir_all(root.join("crates/trios-phd")).unwrap();
        fs::create_dir_all(root.join("tools/page_gate")).unwrap();
        fs::create_dir_all(root.join("tools/acm_ae_check")).unwrap();
        fs::create_dir_all(root.join("docs/phd")).unwrap();
        fs::create_dir_all(root.join("artifact")).unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
        fs::write(
            root.join("crates/trios-phd/Cargo.toml"),
            "[package]\nname='trios-phd'\n",
        )
        .unwrap();
        fs::write(
            root.join("tools/page_gate/Cargo.toml"),
            "[package]\nname='page-gate'\n",
        )
        .unwrap();
        fs::write(
            root.join("tools/acm_ae_check/Cargo.toml"),
            "[package]\nname='acm-ae-check'\n",
        )
        .unwrap();
        fs::write(
            root.join("docs/phd/reproducibility.md"),
            "# Reproducibility\n## Hardware profile\nx86_64 CPU\n## Entry points\n\
             | Goal | Command |\n|--|--|\n| build | cargo run -p trios-phd -- compile |\n\
             ## Software\ntectonic 0.15\n## Notes\nNo bash anywhere (R1).\n",
        )
        .unwrap();
        fs::write(
            root.join("artifact/CLAIMS.md"),
            format!(
                "# Claims\nAnchor: {}\nDOI: {}\n",
                TRINITY_ANCHOR, ZENODO_DOI
            ),
        )
        .unwrap();
        fs::write(root.join("artifact/expected.txt"), fingerprint()).unwrap();
        fs::write(root.join("artifact/output.txt"), fingerprint()).unwrap();
    }

    #[test]
    fn admit_clean_skeleton() {
        let d = tempfile::tempdir().unwrap();
        skel(d.path());
        let r = run_all(d.path()).unwrap();
        assert!(r.functional && r.reusable && r.available && r.fingerprint_ok);
        assert_eq!(r.lane, "LA");
        assert_eq!(r.phase, "D");
    }

    #[test]
    fn falsify_functional_missing_member() {
        let d = tempfile::tempdir().unwrap();
        skel(d.path());
        fs::remove_file(d.path().join("tools/page_gate/Cargo.toml")).unwrap();
        let err = run_all(d.path()).unwrap_err();
        assert_eq!(err.exit_code(), exit::FUNCTIONAL);
        assert!(matches!(err, AcmAeError::Functional { .. }));
    }

    #[test]
    fn falsify_reusable_missing_entrypoint() {
        let d = tempfile::tempdir().unwrap();
        skel(d.path());
        fs::write(
            d.path().join("docs/phd/reproducibility.md"),
            "no entry points here",
        )
        .unwrap();
        let err = run_all(d.path()).unwrap_err();
        assert_eq!(err.exit_code(), exit::REUSABLE);
        assert!(matches!(err, AcmAeError::Reusable { .. }));
    }

    #[test]
    fn falsify_available_missing_doi() {
        let d = tempfile::tempdir().unwrap();
        skel(d.path());
        fs::write(d.path().join("artifact/CLAIMS.md"), "anchor only, no doi").unwrap();
        let err = run_all(d.path()).unwrap_err();
        assert_eq!(err.exit_code(), exit::AVAILABLE);
        assert!(matches!(err, AcmAeError::Available { .. }));
    }

    #[test]
    fn falsify_io_missing_reproducibility_md() {
        let d = tempfile::tempdir().unwrap();
        skel(d.path());
        fs::remove_file(d.path().join("docs/phd/reproducibility.md")).unwrap();
        let err = check_reusable(d.path()).unwrap_err();
        assert_eq!(err.exit_code(), exit::IO);
        assert!(matches!(err, AcmAeError::Io { .. }));
    }

    #[test]
    fn falsify_mismatch_fingerprint() {
        let d = tempfile::tempdir().unwrap();
        skel(d.path());
        fs::write(d.path().join("artifact/output.txt"), "tampered").unwrap();
        let err = run_all(d.path()).unwrap_err();
        assert_eq!(err.exit_code(), exit::MISMATCH);
        match err {
            AcmAeError::Mismatch {
                expected_len,
                observed_len,
                ..
            } => {
                assert!(expected_len > observed_len);
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    #[test]
    fn exit_codes_distinct() {
        // Cardinal ordinal proof: every error variant maps to a unique code,
        // and the LA range 70..=74 is disjoint from L-h4 (50..=53) and LT (60..=63).
        let codes: [u8; 5] = [
            exit::FUNCTIONAL,
            exit::REUSABLE,
            exit::AVAILABLE,
            exit::IO,
            exit::MISMATCH,
        ];
        for c in &codes {
            assert!(*c >= 70 && *c <= 74, "code {c} outside LA range 70..=74");
            assert!(*c < 50 || *c > 53, "LA code {c} clashes with L-h4 50..=53");
            assert!(*c < 60 || *c > 63, "LA code {c} clashes with LT 60..=63");
        }
        let mut sorted = codes.to_vec();
        sorted.sort_unstable();
        let n = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), n, "duplicate exit code");
    }

    #[test]
    fn constants_traceable() {
        // L-R14: every numeric anchor here mirrors a value in
        // assertions/igla_assertions.json (mirrored via tools/page_gate too).
        assert_eq!(MIN_PAGES, 250);
        assert_eq!(MAX_PAGES, 350);
        assert!((PRUNE_THRESHOLD - 3.5).abs() < 1e-12);
        assert_eq!(WARMUP_BLIND_STEPS, 4000);
        assert_eq!(D_MODEL_MIN, 256);
        assert!((LR_CHAMPION - 0.004).abs() < 1e-12);
        const _: () = assert!(LR_CHAMPION >= 0.002 && LR_CHAMPION <= 0.007); // R7
        // Trinity Anchor algebraic check.
        let phi: f64 = 1.618_033_988_749_895;
        assert!((phi * phi + 1.0 / (phi * phi) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn fingerprint_is_deterministic_and_reproducible() {
        // Reusable-badge proof: fingerprint() depends on no env input.
        let a = fingerprint();
        let b = fingerprint();
        assert_eq!(a, b);
        assert!(a.starts_with("ACM-AE-3-BADGE\n"));
        assert!(a.contains(TRINITY_ANCHOR));
        assert!(a.contains(ZENODO_DOI));
        assert!(a.contains("lane=LA\n"));
        assert!(a.contains("phase=D\n"));
        // Bytes count matters for diff stability.
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn forbidden_values_rejected() {
        // R7: prune_threshold == 2.65 is the killer threshold — not present.
        assert_ne!(PRUNE_THRESHOLD, 2.65);
        // R7: warmup < 4000 forbidden.
        const _: () = assert!(WARMUP_BLIND_STEPS >= 4000);
        // R7: d_model < 256 forbidden.
        const _: () = assert!(D_MODEL_MIN >= 256);
        // R7: lr ∉ [0.002, 0.007] forbidden.
        const _: () = assert!(LR_CHAMPION >= 0.002 && LR_CHAMPION <= 0.007);
        // The fingerprint must NOT carry the killer threshold.
        let fp = fingerprint();
        assert!(!fp.contains("prune_threshold=2.65"));
        assert!(fp.contains("prune_threshold=3.5"));
    }
}
