//! SR-HACK-00 — vocabulary glossary
//!
//! Dependency-free, serde-only ring that pins the L0 partnership-plan
//! vocabulary into Rust. Every outreach artefact (DM, PR comment,
//! leaderboard row, Discord post) MUST consume terms from here, never
//! re-invent free strings.
//!
//! Closes #447 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## Rules (ABSOLUTE)
//!
//! - R1  — Pure Rust only (serde derive)
//! - L6  — No I/O, no async, no subprocess, no network
//! - L13 — I-SCOPE: this ring only
//! - R-RING-DEP-002 — deps limited to `serde + serde_json` (test only)

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Ring tier in the metal hierarchy.
///
/// `Gold` = a crate (top layer). `Silver` = a sub-ring inside a crate.
/// `Bronze` = utility / launcher / xtask. `ColorVariant` is reserved
/// for future extensions (e.g. `Platinum`, `Copper`) without breaking
/// roundtrip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RingTier {
    /// 🥇 Gold — top-level crate.
    Gold,
    /// 🥈 Silver — sub-ring inside a Gold crate.
    Silver,
    /// 🥉 Bronze — utility / launcher / xtask.
    Bronze,
    /// Reserved for future tier names. Free-form string.
    ColorVariant(String),
}

impl fmt::Display for RingTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RingTier::Gold => write!(f, "Gold"),
            RingTier::Silver => write!(f, "Silver"),
            RingTier::Bronze => write!(f, "Bronze"),
            RingTier::ColorVariant(s) => write!(f, "{}", s),
        }
    }
}

/// Lane in the IGLA race partnership plan.
///
/// L1..L5 lanes (algorithm, TTT-LoRA, quantization, megakernels, theory).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    /// L-T1 — algorithmic entries (BPB, head shape, optimisers).
    Algorithm,
    /// L-T2 — TTT-LoRA / online adaptation.
    TttLora,
    /// L-T3 — quantization / int8 / fp4 / mixed precision.
    Quantization,
    /// L-T4 — megakernels / fused attention / flash variants.
    Megakernels,
    /// L-T5 — theoretical / proof-bearing entries.
    Theory,
}

impl fmt::Display for Lane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Lane::Algorithm => "Algorithm",
            Lane::TttLora => "TTT-LoRA",
            Lane::Quantization => "Quantization",
            Lane::Megakernels => "Megakernels",
            Lane::Theory => "Theory",
        };
        write!(f, "{}", s)
    }
}

/// Gate threshold checkpoint (G1 = entry, G2 = honest, G3 = champion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Gate {
    /// G1 — first-pass acceptance (BPB < 2.5).
    G1,
    /// G2 — honest gate (BPB < 1.85).
    G2,
    /// G3 — champion gate (BPB < 1.5).
    G3,
}

impl fmt::Display for Gate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gate::G1 => write!(f, "G1"),
            Gate::G2 => write!(f, "G2"),
            Gate::G3 => write!(f, "G3"),
        }
    }
}

/// Canonical vocabulary term.
///
/// Every PR comment, DM, leaderboard row, and Discord post that wants
/// to refer to a structural concept MUST use a `Term` variant — never
/// a free string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Term {
    /// `PipelineO(1)` — the E2E TTT pipeline whose per-chunk cost is
    /// independent of context length.
    PipelineO1,
    /// An entry submitted to the algorithm arena (one row in
    /// `algorithm_arena.jsonl`).
    AlgorithmEntry,
    /// One of the five competitive lanes.
    Lane(Lane),
    /// A gate threshold.
    Gate(Gate),
    /// A ring tier.
    RingTier(RingTier),
    /// `KINGDOM` — top-of-stack governance entity (LAWS, Constitution).
    Kingdom,
    /// `I-SCOPE` — invariant: agent works only inside its own crate.
    IScope,
    /// `SR` — silver ring inside a Gold crate.
    SilverRing,
    /// `BR` — bronze ring (utility / xtask).
    BronzeRing,
    /// `INV` — invariant (numbered, e.g. INV-1..INV-13).
    Invariant,
    /// `LAWS` — `LAWS.md` v2.0 governing the workspace.
    Laws,
    /// `Agent` — autonomous worker bound to one ring.
    Agent,
    /// `Codename` — Greek/Coptic identifier (ALPHA..SHO, OMEGA = LEAD).
    Codename,
    /// `Soul-name` — human-readable agent persona (e.g. "Vocab Vigilante").
    SoulName,
    /// `HEARTBEAT` — periodic liveness signal from a worker to LEAD.
    Heartbeat,
    /// `DONE` — terminal state of a ring task.
    Done,
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::PipelineO1 => write!(f, "PipelineO(1)"),
            Term::AlgorithmEntry => write!(f, "AlgorithmEntry"),
            Term::Lane(l) => write!(f, "Lane({})", l),
            Term::Gate(g) => write!(f, "Gate({})", g),
            Term::RingTier(t) => write!(f, "RingTier({})", t),
            Term::Kingdom => write!(f, "KINGDOM"),
            Term::IScope => write!(f, "I-SCOPE"),
            Term::SilverRing => write!(f, "SR"),
            Term::BronzeRing => write!(f, "BR"),
            Term::Invariant => write!(f, "INV"),
            Term::Laws => write!(f, "LAWS"),
            Term::Agent => write!(f, "Agent"),
            Term::Codename => write!(f, "Codename"),
            Term::SoulName => write!(f, "Soul-name"),
            Term::Heartbeat => write!(f, "HEARTBEAT"),
            Term::Done => write!(f, "DONE"),
        }
    }
}

/// Returns every canonical `Term` for enumeration / completeness tests.
///
/// Used by SR-HACK-01..05 to guarantee they cover every term in DMs
/// and audit reports.
pub fn all_terms() -> Vec<Term> {
    vec![
        Term::PipelineO1,
        Term::AlgorithmEntry,
        Term::Lane(Lane::Algorithm),
        Term::Lane(Lane::TttLora),
        Term::Lane(Lane::Quantization),
        Term::Lane(Lane::Megakernels),
        Term::Lane(Lane::Theory),
        Term::Gate(Gate::G1),
        Term::Gate(Gate::G2),
        Term::Gate(Gate::G3),
        Term::RingTier(RingTier::Gold),
        Term::RingTier(RingTier::Silver),
        Term::RingTier(RingTier::Bronze),
        Term::Kingdom,
        Term::IScope,
        Term::SilverRing,
        Term::BronzeRing,
        Term::Invariant,
        Term::Laws,
        Term::Agent,
        Term::Codename,
        Term::SoulName,
        Term::Heartbeat,
        Term::Done,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_tier_roundtrip() {
        for t in [
            RingTier::Gold,
            RingTier::Silver,
            RingTier::Bronze,
            RingTier::ColorVariant("Platinum".into()),
        ] {
            let s = serde_json::to_string(&t).unwrap();
            let back: RingTier = serde_json::from_str(&s).unwrap();
            assert_eq!(t, back, "RingTier roundtrip mismatch on {:?}", t);
        }
    }

    #[test]
    fn lane_roundtrip() {
        for l in [
            Lane::Algorithm,
            Lane::TttLora,
            Lane::Quantization,
            Lane::Megakernels,
            Lane::Theory,
        ] {
            let s = serde_json::to_string(&l).unwrap();
            let back: Lane = serde_json::from_str(&s).unwrap();
            assert_eq!(l, back);
        }
    }

    #[test]
    fn gate_roundtrip() {
        for g in [Gate::G1, Gate::G2, Gate::G3] {
            let s = serde_json::to_string(&g).unwrap();
            let back: Gate = serde_json::from_str(&s).unwrap();
            assert_eq!(g, back);
        }
    }

    #[test]
    fn lane_serializes_snake_case() {
        let s = serde_json::to_string(&Lane::TttLora).unwrap();
        assert_eq!(s, "\"ttt_lora\"");
    }

    #[test]
    fn gate_serializes_uppercase() {
        let s = serde_json::to_string(&Gate::G2).unwrap();
        assert_eq!(s, "\"G2\"");
    }

    #[test]
    fn ring_tier_color_variant_roundtrip() {
        let t = RingTier::ColorVariant("Cobalt".into());
        let s = serde_json::to_string(&t).unwrap();
        let back: RingTier = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn term_roundtrip_every_variant() {
        for term in all_terms() {
            let s = serde_json::to_string(&term).unwrap();
            let back: Term = serde_json::from_str(&s).unwrap();
            assert_eq!(term, back, "Term roundtrip mismatch on {:?}", term);
        }
    }

    #[test]
    fn all_terms_at_least_15() {
        // Acceptance criterion from #447: ≥ 15 canonical terms exposed.
        assert!(
            all_terms().len() >= 15,
            "expected ≥ 15 terms, got {}",
            all_terms().len()
        );
    }

    #[test]
    fn display_strings_are_stable() {
        // Stable wire format that PR comments / DMs / Discord depend on.
        assert_eq!(format!("{}", Term::PipelineO1), "PipelineO(1)");
        assert_eq!(format!("{}", Term::AlgorithmEntry), "AlgorithmEntry");
        assert_eq!(
            format!("{}", Term::Lane(Lane::TttLora)),
            "Lane(TTT-LoRA)"
        );
        assert_eq!(format!("{}", Term::Gate(Gate::G2)), "Gate(G2)");
        assert_eq!(
            format!("{}", Term::RingTier(RingTier::Gold)),
            "RingTier(Gold)"
        );
        assert_eq!(format!("{}", Term::IScope), "I-SCOPE");
        assert_eq!(format!("{}", Term::Kingdom), "KINGDOM");
    }

    #[test]
    fn term_tagged_kind_format() {
        // serde tag = "kind" — every Term JSON object MUST have a "kind" field.
        let s = serde_json::to_string(&Term::PipelineO1).unwrap();
        assert!(s.contains("\"kind\""), "missing 'kind' tag in {}", s);
    }
}
