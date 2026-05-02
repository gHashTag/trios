//! SR-MEM-00 — memory-types
//!
//! Anti-amnesia foundation. Dependency-free typed primitives for the
//! agent-memory layer; every backend (Neon `lessons`, `trios-kg`,
//! `zig-knowledge-graph`, HDC episodic agents) speaks these types.
//!
//! Closes #449 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## Public types
//!
//! | Type              | Wire-format role |
//! |-------------------|------------------|
//! | `TripleId([u8;32])` | content-addressed (SHA-256 of subject+predicate+object) |
//! | `Provenance`      | `agent_id`, `task_id`, `source_sha`, `ts` |
//! | `AgentRole`       | enum: Scarab, Gardener, Trainer, Doctor, Claude, Lead |
//! | `MemoryKind`      | enum: Working, Session, LongTerm, Episodic, Semantic |
//! | `ForgetPolicy`    | enum: GdprByAgent, AgeOlderThan, PredicateMatches |
//! | `Triple`          | `(subject, predicate, object, provenance)` |
//!
//! ## Rules
//!
//! - R1   — pure Rust
//! - L6   — no I/O, no async
//! - L13  — I-SCOPE: only this ring
//! - R-RING-DEP-002 — deps = `serde + serde_json + uuid + chrono + sha2`

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ───────────── TripleId ─────────────

/// Content-addressed triple identifier — SHA-256 of `subject || predicate || object`.
///
/// JSON serialisation is lowercase hex (length 64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TripleId(pub [u8; 32]);

impl TripleId {
    /// Build by hashing `subject || predicate || object` (no separator —
    /// this is the canonical content-address). Idempotent for fixed inputs.
    pub fn from_triple(subject: &str, predicate: &str, object: &str) -> Self {
        let mut h = Sha256::new();
        h.update(subject.as_bytes());
        h.update(predicate.as_bytes());
        h.update(object.as_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        Self(out)
    }

    /// Underlying bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for TripleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl Serialize for TripleId {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TripleId {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        if s.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "TripleId hex must be 64 chars, got {}",
                s.len()
            )));
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            let pair = &s[i * 2..i * 2 + 2];
            *byte = u8::from_str_radix(pair, 16).map_err(serde::de::Error::custom)?;
        }
        Ok(Self(out))
    }
}

// ───────────── AgentRole ─────────────

/// Which agent class produced this memory.
///
/// Mirrors the canonical Trinity codename roles. JSON is snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Scarab — trainer worker.
    Scarab,
    /// Gardener — ASHA pruner.
    Gardener,
    /// Trainer — orchestration of a `Scarab`.
    Trainer,
    /// Doctor — health / lint surface.
    Doctor,
    /// Claude — conversational reasoning agent.
    Claude,
    /// LEAD — top-of-stack governance.
    Lead,
}

// ───────────── MemoryKind ─────────────

/// Memory tier (CMU Cognitive Architecture taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// Working — within a single tick, evicted at end-of-tick.
    Working,
    /// Session — within one trainer session.
    Session,
    /// LongTerm — persisted across sessions.
    LongTerm,
    /// Episodic — concrete past events with timestamp + provenance.
    Episodic,
    /// Semantic — distilled facts / lessons.
    Semantic,
}

// ───────────── ForgetPolicy ─────────────

/// What MUST be forgotten and when.
///
/// JSON variants are tagged with `kind` for safe extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ForgetPolicy {
    /// GDPR forget request keyed on agent role (e.g. "forget all
    /// `Claude`-authored triples").
    GdprByAgent {
        /// Which role to forget.
        agent: AgentRole,
    },
    /// Age-based forgetting (`now - ts > duration`).
    AgeOlderThan {
        /// Cut-off duration (serialises as integer seconds).
        #[serde(with = "duration_secs")]
        duration: Duration,
    },
    /// Predicate-based selective forgetting.
    PredicateMatches {
        /// Predicate to match (string equality).
        predicate: String,
    },
}

mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u64(d.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(de)?;
        Ok(Duration::from_secs(secs))
    }
}

// ───────────── Provenance ─────────────

/// Where a memory came from. Required on every persistent triple so
/// audit trails (and GDPR forget) can target the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Agent that produced the memory.
    pub agent_id: AgentRole,
    /// Task id (free-form, typically a JobId or run id).
    pub task_id: Uuid,
    /// Source SHA — git commit / model checkpoint that authored this
    /// triple (32-byte SHA-256, lowercase hex).
    pub source_sha: TripleId,
    /// When the memory was minted (UTC).
    pub ts: DateTime<Utc>,
}

// ───────────── Triple ─────────────

/// One memory atom: `(subject, predicate, object)` with provenance.
///
/// `id` is content-addressed (`TripleId::from_triple`). Two triples
/// with identical SPO produce the same id (idempotent insert).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Triple {
    /// Content-addressed identifier.
    pub id: TripleId,
    /// Subject string.
    pub subject: String,
    /// Predicate string.
    pub predicate: String,
    /// Object string.
    pub object: String,
    /// Provenance + timing.
    pub provenance: Provenance,
}

impl Triple {
    /// Build a triple, computing `id` from SPO. Idempotent.
    pub fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        let s = subject.into();
        let p = predicate.into();
        let o = object.into();
        let id = TripleId::from_triple(&s, &p, &o);
        Self {
            id,
            subject: s,
            predicate: p,
            object: o,
            provenance,
        }
    }
}

// ───────────── tests ─────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 9, 0, 0).unwrap()
    }

    fn dummy_provenance() -> Provenance {
        Provenance {
            agent_id: AgentRole::Lead,
            task_id: Uuid::new_v4(),
            source_sha: TripleId::from_triple("commit", "abcdef", ""),
            ts: fixed_ts(),
        }
    }

    #[test]
    fn triple_id_is_content_addressed() {
        let a = TripleId::from_triple("S", "P", "O");
        let b = TripleId::from_triple("S", "P", "O");
        assert_eq!(a, b, "same SPO must yield same id");
    }

    #[test]
    fn triple_id_distinguishes_inputs() {
        let a = TripleId::from_triple("S1", "P", "O");
        let b = TripleId::from_triple("S2", "P", "O");
        assert_ne!(a, b);
    }

    #[test]
    fn triple_id_serialises_as_hex64() {
        let id = TripleId::from_triple("alice", "knows", "bob");
        let s = serde_json::to_string(&id).unwrap();
        assert_eq!(s.len(), 66, "expected 64-char hex string in quotes, got {}", s);
        let back: TripleId = serde_json::from_str(&s).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn triple_id_rejects_wrong_length() {
        let s = format!("\"{}\"", "ab".repeat(31));
        assert!(serde_json::from_str::<TripleId>(&s).is_err());
    }

    #[test]
    fn triple_id_display_lowercase_hex() {
        let id = TripleId::from_triple("a", "b", "c");
        let d = format!("{}", id);
        assert_eq!(d.len(), 64);
        assert!(d.chars().all(|c| c.is_ascii_hexdigit() && (!c.is_alphabetic() || c.is_lowercase())));
    }

    #[test]
    fn agent_role_serialises_snake_case() {
        for (r, exp) in [
            (AgentRole::Scarab, "\"scarab\""),
            (AgentRole::Gardener, "\"gardener\""),
            (AgentRole::Lead, "\"lead\""),
        ] {
            assert_eq!(serde_json::to_string(&r).unwrap(), exp);
        }
    }

    #[test]
    fn memory_kind_roundtrip() {
        for k in [
            MemoryKind::Working,
            MemoryKind::Session,
            MemoryKind::LongTerm,
            MemoryKind::Episodic,
            MemoryKind::Semantic,
        ] {
            let s = serde_json::to_string(&k).unwrap();
            let back: MemoryKind = serde_json::from_str(&s).unwrap();
            assert_eq!(k, back);
        }
    }

    #[test]
    fn memory_kind_long_term_is_snake_case() {
        let s = serde_json::to_string(&MemoryKind::LongTerm).unwrap();
        assert_eq!(s, "\"long_term\"");
    }

    #[test]
    fn forget_policy_gdpr_roundtrip() {
        let p = ForgetPolicy::GdprByAgent {
            agent: AgentRole::Claude,
        };
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.contains("\"kind\""));
        let back: ForgetPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn forget_policy_age_serializes_as_seconds() {
        let p = ForgetPolicy::AgeOlderThan {
            duration: Duration::from_secs(3600),
        };
        let s = serde_json::to_string(&p).unwrap();
        // expect "duration":3600
        assert!(s.contains("\"duration\":3600"), "got {}", s);
        let back: ForgetPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn forget_policy_predicate_roundtrip() {
        let p = ForgetPolicy::PredicateMatches {
            predicate: "knows".into(),
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: ForgetPolicy = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn provenance_roundtrip() {
        let pr = dummy_provenance();
        let s = serde_json::to_string(&pr).unwrap();
        let back: Provenance = serde_json::from_str(&s).unwrap();
        assert_eq!(pr, back);
    }

    #[test]
    fn triple_new_computes_id() {
        let t = Triple::new("alice", "knows", "bob", dummy_provenance());
        let expect = TripleId::from_triple("alice", "knows", "bob");
        assert_eq!(t.id, expect);
    }

    #[test]
    fn triple_idempotent_insert() {
        // Two triples with the same SPO and the *same* provenance must
        // be byte-equal — content-addressed insert is idempotent.
        let pr = dummy_provenance();
        let t1 = Triple::new("alice", "knows", "bob", pr.clone());
        let t2 = Triple::new("alice", "knows", "bob", pr);
        assert_eq!(t1, t2);
    }

    #[test]
    fn triple_full_roundtrip() {
        let t = Triple::new("alice", "knows", "bob", dummy_provenance());
        let s = serde_json::to_string(&t).unwrap();
        let back: Triple = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }
}
