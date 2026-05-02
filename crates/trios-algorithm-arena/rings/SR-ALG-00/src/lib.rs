//! SR-ALG-00 — arena-types
//!
//! Dependency-free typed metadata for one algorithm submitted to the
//! TRIOS IGLA arena. This is the bottom of the GOLD II dependency
//! graph; SR-ALG-01..03 + BR-OUTPUT all import their wire format from
//! here.
//!
//! Closes #450 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## Critical: no Python in `crates/`
//!
//! `AlgorithmSpec::entry_path` points *out* of `crates/` (typically
//! into `parameter-golf/records/...`). Real Python spawn is the job of
//! SR-02 trainer-runner — never this crate. R-L6-PURE-007.
//!
//! ## Public types
//!
//! | Type            | Wire-format role |
//! |-----------------|------------------|
//! | `AlgorithmId`   | UUID v4 newtype identifying one algorithm |
//! | `EntryHash`     | `[u8; 32]` SHA-256 of the entry script source |
//! | `EnvVar`        | environment variable name newtype |
//! | `EnvValue`      | environment variable value newtype |
//! | `GoldenState`   | `[u8; 32]` SHA-256 of the convergence checkpoint |
//! | `AlgorithmSpec` | full manifest |
//!
//! ## Rules
//!
//! - R1   — pure Rust
//! - L6   — no I/O, no async, no subprocess
//! - L13  — I-SCOPE: only this ring
//! - R-RING-DEP-002 — deps = `serde + uuid + hex` (+ `serde_json` dev)
//! - R-L6-PURE-007 — no `.py` in this crate; entry_path is a *reference*

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────── newtypes ─────────────

/// Unique identifier of one algorithm submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AlgorithmId(pub Uuid);

impl AlgorithmId {
    /// Generate a fresh v4 id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    /// Underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}
impl Default for AlgorithmId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for AlgorithmId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Environment variable name (e.g. `OPTIMIZER`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnvVar(pub String);

impl EnvVar {
    /// Build from any string-like input.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// Backing string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for EnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Environment variable value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnvValue(pub String);

impl EnvValue {
    /// Build from any string-like input.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// Backing string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for EnvValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ───────────── 32-byte hashes ─────────────

/// SHA-256 of the entry script source (post-normalisation).
///
/// JSON serialisation is lowercase hex (length 64) for human-readable
/// arena manifests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntryHash(pub [u8; 32]);

impl EntryHash {
    /// Underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    /// Compare to a candidate `[u8; 32]`.
    pub fn matches(&self, other: &[u8; 32]) -> bool {
        &self.0 == other
    }
}

impl fmt::Display for EntryHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl Serialize for EntryHash {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for EntryHash {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "EntryHash must be 32 bytes (64 hex chars), got {}",
                bytes.len()
            )));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

/// SHA-256 of the convergence checkpoint a contestant must reach.
///
/// Serialises as lowercase hex. Optional in [`AlgorithmSpec`] —
/// theory-only entries ship without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GoldenState(pub [u8; 32]);

impl GoldenState {
    /// Underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for GoldenState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl Serialize for GoldenState {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for GoldenState {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom(format!(
                "GoldenState must be 32 bytes (64 hex chars), got {}",
                bytes.len()
            )));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

// ───────────── AlgorithmSpec ─────────────

/// Manifest for one algorithm submitted to the IGLA arena.
///
/// `entry_path` MUST point *outside* `crates/` (typically into
/// `parameter-golf/records/...`). This crate never executes Python.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlgorithmSpec {
    /// Unique id of this submission.
    pub algorithm_id: AlgorithmId,
    /// Human-readable name (`"e2e-ttt"`, `"jepa-12k"`, …).
    pub name: String,
    /// Filesystem path to the entry script (NEVER inside `crates/`).
    pub entry_path: PathBuf,
    /// SHA-256 of the entry script source (post-normalisation).
    pub entry_hash: EntryHash,
    /// Ordered list of `(env_var, env_value)` pairs.
    pub env: Vec<(EnvVar, EnvValue)>,
    /// Optional convergence checkpoint hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub golden_state_hash: Option<GoldenState>,
    /// Optional theorem citation (e.g. `"Coq.Reals Thm 3.1 SAC-1"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theorem: Option<String>,
}

impl AlgorithmSpec {
    /// Cheap convenience constructor for the common case
    /// (`name`, `entry_path`, `entry_hash`).
    pub fn new(name: impl Into<String>, entry_path: PathBuf, entry_hash: EntryHash) -> Self {
        Self {
            algorithm_id: AlgorithmId::new(),
            name: name.into(),
            entry_path,
            entry_hash,
            env: Vec::new(),
            golden_state_hash: None,
            theorem: None,
        }
    }

    /// Compare an actual SHA-256 to the manifest's [`EntryHash`].
    pub fn verify_hash(&self, actual: &[u8; 32]) -> bool {
        self.entry_hash.matches(actual)
    }
}

// ───────────── tests ─────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_32() -> [u8; 32] {
        // 0x00..0x1f
        let mut out = [0u8; 32];
        for (i, b) in out.iter_mut().enumerate() {
            *b = i as u8;
        }
        out
    }

    fn other_32() -> [u8; 32] {
        let mut out = [0u8; 32];
        for (i, b) in out.iter_mut().enumerate() {
            *b = (i as u8).wrapping_add(0x55);
        }
        out
    }

    #[test]
    fn algorithm_id_unique_per_instance() {
        let a = AlgorithmId::new();
        let b = AlgorithmId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn entry_hash_32_bytes_serialises_as_hex64() {
        let h = EntryHash(fixed_32());
        let s = serde_json::to_string(&h).unwrap();
        // 64 hex chars + 2 quotes
        assert_eq!(s.len(), 66, "expected 64-char hex string, got {}", s);
        let back: EntryHash = serde_json::from_str(&s).unwrap();
        assert_eq!(h, back);
    }

    #[test]
    fn entry_hash_rejects_wrong_length() {
        // 31 bytes → 62 hex chars → must fail.
        let s = format!("\"{}\"", "ab".repeat(31));
        assert!(serde_json::from_str::<EntryHash>(&s).is_err());
    }

    #[test]
    fn golden_state_serialises_as_hex64() {
        let g = GoldenState(fixed_32());
        let s = serde_json::to_string(&g).unwrap();
        let back: GoldenState = serde_json::from_str(&s).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn env_empty_vec_valid() {
        let spec = AlgorithmSpec::new(
            "noop",
            PathBuf::from("parameter-golf/records/noop/train.py"),
            EntryHash(fixed_32()),
        );
        assert!(spec.env.is_empty());
        let s = serde_json::to_string(&spec).unwrap();
        let back: AlgorithmSpec = serde_json::from_str(&s).unwrap();
        assert_eq!(spec, back);
    }

    #[test]
    fn entry_path_from_str_round_trips() {
        let path = PathBuf::from("parameter-golf/records/e2e-ttt/train.py");
        let spec = AlgorithmSpec::new("e2e-ttt", path.clone(), EntryHash(fixed_32()));
        assert_eq!(spec.entry_path, path);
    }

    #[test]
    fn theorem_optional() {
        let spec = AlgorithmSpec::new(
            "no-theory",
            PathBuf::from("parameter-golf/records/no-theory/train.py"),
            EntryHash(fixed_32()),
        );
        assert!(spec.theorem.is_none());
        let s = serde_json::to_string(&spec).unwrap();
        assert!(!s.contains("\"theorem\""), "theorem must be omitted when None");
    }

    #[test]
    fn golden_state_hash_optional() {
        let spec = AlgorithmSpec::new(
            "no-state",
            PathBuf::from("parameter-golf/records/no-state/train.py"),
            EntryHash(fixed_32()),
        );
        assert!(spec.golden_state_hash.is_none());
        let s = serde_json::to_string(&spec).unwrap();
        assert!(
            !s.contains("\"golden_state_hash\""),
            "golden_state_hash must be omitted when None"
        );
    }

    #[test]
    fn verify_hash_correct() {
        let h = fixed_32();
        let spec = AlgorithmSpec::new(
            "x",
            PathBuf::from("parameter-golf/records/x/train.py"),
            EntryHash(h),
        );
        assert!(spec.verify_hash(&h));
    }

    #[test]
    fn verify_hash_wrong_fails() {
        let spec = AlgorithmSpec::new(
            "x",
            PathBuf::from("parameter-golf/records/x/train.py"),
            EntryHash(fixed_32()),
        );
        assert!(!spec.verify_hash(&other_32()));
    }

    #[test]
    fn serde_roundtrip_full_spec() {
        let mut h = [0u8; 32];
        for (i, b) in h.iter_mut().enumerate() {
            *b = i as u8;
        }
        let spec = AlgorithmSpec {
            algorithm_id: AlgorithmId::new(),
            name: "e2e-ttt".into(),
            entry_path: PathBuf::from("parameter-golf/records/e2e-ttt/train.py"),
            entry_hash: EntryHash(h),
            env: vec![
                (EnvVar::new("OPTIMIZER"), EnvValue::new("AdamW")),
                (EnvVar::new("LR"), EnvValue::new("0.005")),
            ],
            golden_state_hash: Some(GoldenState(h)),
            theorem: Some("Coq.Reals Theorem 3.1 SAC-1".into()),
        };
        let s = serde_json::to_string(&spec).unwrap();
        let back: AlgorithmSpec = serde_json::from_str(&s).unwrap();
        assert_eq!(spec, back);
    }

    #[test]
    fn schema_field_names_stable() {
        // Wire-format guard: SR-ALG-01..03 + BR-OUTPUT depend on these
        // exact JSON key names. Renaming any field MUST be paired with
        // an audit of every consumer.
        let spec = AlgorithmSpec::new(
            "x",
            PathBuf::from("parameter-golf/records/x/train.py"),
            EntryHash(fixed_32()),
        );
        let v: serde_json::Value = serde_json::to_value(&spec).unwrap();
        for k in [
            "algorithm_id",
            "name",
            "entry_path",
            "entry_hash",
            "env",
        ] {
            assert!(
                v.get(k).is_some(),
                "AlgorithmSpec field '{}' missing from JSON",
                k
            );
        }
    }
}
