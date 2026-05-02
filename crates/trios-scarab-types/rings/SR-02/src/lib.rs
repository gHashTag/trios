//! SR-02 scarab-soul ring.
//!
//! Combines O-Type + Three-Type for soul-level agents.
//!
//! Closes #479 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! Dep budget (R-RING-DEP-002): `serde` + SR-00 sibling only.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use serde::{Deserialize, Serialize};
use trios_scarab_types_sr_00::Term;

/// Variant tag for SoulScarab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoulScarabType {
    /// O-Type carrier (origin / monad term).
    OType,
    /// Three-Type carrier.
    ThreeType,
    /// Composite — both O-Type and Three-Type are bound.
    Composite,
}

impl SoulScarabType {
    /// Stable string slug.
    pub fn slug(self) -> &'static str {
        match self {
            SoulScarabType::OType => "SR-02/O-Type",
            SoulScarabType::ThreeType => "SR-02/Three-Type",
            SoulScarabType::Composite => "SR-02/Composite",
        }
    }

    /// Markdown bullet representation.
    pub fn as_markdown(self) -> String {
        format!("- **{}**", self.slug())
    }

    /// All variants in declaration order.
    pub fn all() -> [SoulScarabType; 3] {
        [
            SoulScarabType::OType,
            SoulScarabType::ThreeType,
            SoulScarabType::Composite,
        ]
    }
}

impl fmt::Display for SoulScarabType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// SoulScarab — soul-level scarab. Carries a `SoulScarabType` and the
/// underlying SR-00 `Term` pair (O-Type, ThreeType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SoulScarab {
    /// Variant tag.
    pub kind: SoulScarabType,
    /// O-Type term (always bound).
    pub o_term: Term,
    /// Three-Type term (always bound).
    pub compound_term: Term,
}

impl SoulScarab {
    /// Build a fresh soul-level scarab.
    pub fn new(kind: SoulScarabType) -> Self {
        Self {
            kind,
            o_term: Term::OType,
            compound_term: Term::ThreeType,
        }
    }
}

impl fmt::Display for SoulScarab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SoulScarab({})", self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_variants_present() {
        assert_eq!(SoulScarabType::all().len(), 3);
    }

    #[test]
    fn slugs_are_unique() {
        let v: Vec<&str> = SoulScarabType::all().iter().map(|t| t.slug()).collect();
        let unique: std::collections::HashSet<&str> = v.iter().copied().collect();
        assert_eq!(v.len(), unique.len());
    }

    #[test]
    fn display_matches_slug() {
        assert_eq!(format!("{}", SoulScarabType::OType), "SR-02/O-Type");
        assert_eq!(format!("{}", SoulScarabType::Composite), "SR-02/Composite");
    }

    #[test]
    fn as_markdown_emits_bullet() {
        assert_eq!(SoulScarabType::OType.as_markdown(), "- **SR-02/O-Type**");
    }

    #[test]
    fn struct_carries_term_pair() {
        let s = SoulScarab::new(SoulScarabType::Composite);
        assert_eq!(s.o_term, Term::OType);
        assert_eq!(s.compound_term, Term::ThreeType);
    }

    #[test]
    fn struct_serde_round_trip() {
        let s = SoulScarab::new(SoulScarabType::ThreeType);
        let j = serde_json::to_string(&s).unwrap();
        let back: SoulScarab = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }
}
