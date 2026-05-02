//! SR-01 scarab-lane ring.
//!
//! Combines O-Type + Two-Type for lane-level agents.
//!
//! Closes #479 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! Dep budget (R-RING-DEP-002): `serde` + SR-00 sibling only.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use serde::{Deserialize, Serialize};
use trios_scarab_types_sr_00::Term;

/// Variant tag for LaneScarab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LaneScarabType {
    /// O-Type carrier (origin / monad term).
    OType,
    /// Two-Type carrier.
    TwoType,
    /// Composite — both O-Type and Two-Type are bound.
    Composite,
}

impl LaneScarabType {
    /// Stable string slug.
    pub fn slug(self) -> &'static str {
        match self {
            LaneScarabType::OType => "SR-01/O-Type",
            LaneScarabType::TwoType => "SR-01/Two-Type",
            LaneScarabType::Composite => "SR-01/Composite",
        }
    }

    /// Markdown bullet representation.
    pub fn as_markdown(self) -> String {
        format!("- **{}**", self.slug())
    }

    /// All variants in declaration order.
    pub fn all() -> [LaneScarabType; 3] {
        [
            LaneScarabType::OType,
            LaneScarabType::TwoType,
            LaneScarabType::Composite,
        ]
    }
}

impl fmt::Display for LaneScarabType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// LaneScarab — lane-level scarab. Carries a `LaneScarabType` and the
/// underlying SR-00 `Term` pair (O-Type, TwoType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LaneScarab {
    /// Variant tag.
    pub kind: LaneScarabType,
    /// O-Type term (always bound).
    pub o_term: Term,
    /// Two-Type term (always bound).
    pub compound_term: Term,
}

impl LaneScarab {
    /// Build a fresh lane-level scarab.
    pub fn new(kind: LaneScarabType) -> Self {
        Self {
            kind,
            o_term: Term::OType,
            compound_term: Term::TwoType,
        }
    }
}

impl fmt::Display for LaneScarab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LaneScarab({})", self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_variants_present() {
        assert_eq!(LaneScarabType::all().len(), 3);
    }

    #[test]
    fn slugs_are_unique() {
        let v: Vec<&str> = LaneScarabType::all().iter().map(|t| t.slug()).collect();
        let unique: std::collections::HashSet<&str> = v.iter().copied().collect();
        assert_eq!(v.len(), unique.len());
    }

    #[test]
    fn display_matches_slug() {
        assert_eq!(format!("{}", LaneScarabType::OType), "SR-01/O-Type");
        assert_eq!(format!("{}", LaneScarabType::Composite), "SR-01/Composite");
    }

    #[test]
    fn as_markdown_emits_bullet() {
        assert_eq!(LaneScarabType::OType.as_markdown(), "- **SR-01/O-Type**");
    }

    #[test]
    fn struct_carries_term_pair() {
        let s = LaneScarab::new(LaneScarabType::Composite);
        assert_eq!(s.o_term, Term::OType);
        assert_eq!(s.compound_term, Term::TwoType);
    }

    #[test]
    fn struct_serde_round_trip() {
        let s = LaneScarab::new(LaneScarabType::TwoType);
        let j = serde_json::to_string(&s).unwrap();
        let back: LaneScarab = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }
}
