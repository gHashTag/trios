//! SR-00 — scarab-types
//!
//! 16-variant `Term` enum used as the foundation for every higher-tier
//! scarab category. Each term implements `Display` and `as_markdown`.
//!
//! Closes #479 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## Rules (ABSOLUTE)
//!
//! - R1  — Pure Rust only (`serde`)
//! - L6  — No I/O, no async, no subprocess
//! - L13 — I-SCOPE: this ring only
//! - R-RING-DEP-002 — dep limited to `serde`

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use serde::{Deserialize, Serialize};

/// Core scarab term — the 16-element alphabet over which Lane / Soul /
/// Gate scarabs are composed. The names are the canonical English
/// spellings used in `trinity-bootstrap` and the AGENTS grid (PR #469).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Term {
    /// O-Type — the origin / monad term.
    OType,
    /// One-Type.
    OneType,
    /// Two-Type.
    TwoType,
    /// Three-Type.
    ThreeType,
    /// Four-Type.
    FourType,
    /// Five-Type.
    FiveType,
    /// Six-Type.
    SixType,
    /// Seven-Type.
    SevenType,
    /// Eight-Type.
    EightType,
    /// Nine-Type.
    NineType,
    /// Ten-Type — first compound term beyond the base decade.
    TenType,
    /// Eleven-Type.
    ElevenType,
    /// Twelve-Type.
    TwelveType,
    /// Trinity-Type — the canonical trinity scarab marker.
    TrinityType,
    /// Phi-Type — the φ marker.
    PhiType,
    /// Lucas-Type — the Lucas-orbit marker.
    LucasType,
}

impl Term {
    /// Stable string slug used in JSON serialization and CLI logs.
    pub fn slug(self) -> &'static str {
        match self {
            Term::OType => "O-Type",
            Term::OneType => "One-Type",
            Term::TwoType => "Two-Type",
            Term::ThreeType => "Three-Type",
            Term::FourType => "Four-Type",
            Term::FiveType => "Five-Type",
            Term::SixType => "Six-Type",
            Term::SevenType => "Seven-Type",
            Term::EightType => "Eight-Type",
            Term::NineType => "Nine-Type",
            Term::TenType => "Ten-Type",
            Term::ElevenType => "Eleven-Type",
            Term::TwelveType => "Twelve-Type",
            Term::TrinityType => "Trinity-Type",
            Term::PhiType => "Phi-Type",
            Term::LucasType => "Lucas-Type",
        }
    }

    /// Render this term as a one-line Markdown bullet for AGENTS grid
    /// rendering.
    pub fn as_markdown(self) -> String {
        format!("- **{}**", self.slug())
    }

    /// Every term, in declaration order.
    pub fn all() -> [Term; 16] {
        [
            Term::OType,
            Term::OneType,
            Term::TwoType,
            Term::ThreeType,
            Term::FourType,
            Term::FiveType,
            Term::SixType,
            Term::SevenType,
            Term::EightType,
            Term::NineType,
            Term::TenType,
            Term::ElevenType,
            Term::TwelveType,
            Term::TrinityType,
            Term::PhiType,
            Term::LucasType,
        ]
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// One scarab carrying a single `Term`. The wrapper exists so SR-01..04
/// can pin a phantom-typed scarab onto each kingdom (Lane / Soul /
/// Gate) without leaking the underlying `Term` into the higher-tier
/// API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TermScarab {
    /// The carried term.
    pub term: Term,
}

impl TermScarab {
    /// Build a new term scarab.
    pub fn new(term: Term) -> Self {
        Self { term }
    }
}

impl fmt::Display for TermScarab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TermScarab({})", self.term)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sixteen_terms_present() {
        assert_eq!(Term::all().len(), 16);
    }

    #[test]
    fn term_display_matches_slug() {
        assert_eq!(format!("{}", Term::OType), "O-Type");
        assert_eq!(format!("{}", Term::TrinityType), "Trinity-Type");
        assert_eq!(format!("{}", Term::PhiType), "Phi-Type");
    }

    #[test]
    fn term_as_markdown_emits_bullet() {
        assert_eq!(Term::OType.as_markdown(), "- **O-Type**");
        assert_eq!(Term::NineType.as_markdown(), "- **Nine-Type**");
    }

    #[test]
    fn slugs_are_stable_and_unique() {
        let slugs: Vec<&str> = Term::all().iter().map(|t| t.slug()).collect();
        let unique: std::collections::HashSet<&str> = slugs.iter().copied().collect();
        assert_eq!(slugs.len(), unique.len(), "slugs must be unique");
    }

    #[test]
    fn term_scarab_round_trips_through_serde() {
        let s = TermScarab::new(Term::TrinityType);
        let j = serde_json::to_string(&s).unwrap();
        let back: TermScarab = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }
}
