//! SR-03 scarab-gate-four ring.
//!
//! Combines O-Type + Four-Type for gate-four-level agents.
//!
//! Closes #479 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! Dep budget (R-RING-DEP-002): `serde` + SR-00 sibling only.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;

use serde::{Deserialize, Serialize};
use trios_scarab_types_sr_00::Term;

/// Variant tag for GateScarab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateScarabFourType {
    /// O-Type carrier (origin / monad term).
    OType,
    /// Four-Type carrier.
    FourType,
    /// Composite — both O-Type and Four-Type are bound.
    Composite,
}

impl GateScarabFourType {
    /// Stable string slug.
    pub fn slug(self) -> &'static str {
        match self {
            GateScarabFourType::OType => "SR-03/O-Type",
            GateScarabFourType::FourType => "SR-03/Four-Type",
            GateScarabFourType::Composite => "SR-03/Composite",
        }
    }

    /// Markdown bullet representation.
    pub fn as_markdown(self) -> String {
        format!("- **{}**", self.slug())
    }

    /// All variants in declaration order.
    pub fn all() -> [GateScarabFourType; 3] {
        [
            GateScarabFourType::OType,
            GateScarabFourType::FourType,
            GateScarabFourType::Composite,
        ]
    }
}

impl fmt::Display for GateScarabFourType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// GateScarab — gate-four-level scarab. Carries a `GateScarabFourType` and the
/// underlying SR-00 `Term` pair (O-Type, FourType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GateScarab {
    /// Variant tag.
    pub kind: GateScarabFourType,
    /// O-Type term (always bound).
    pub o_term: Term,
    /// Four-Type term (always bound).
    pub compound_term: Term,
}

impl GateScarab {
    /// Build a fresh gate-four-level scarab.
    pub fn new(kind: GateScarabFourType) -> Self {
        Self {
            kind,
            o_term: Term::OType,
            compound_term: Term::FourType,
        }
    }
}

impl fmt::Display for GateScarab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GateScarab({})", self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_variants_present() {
        assert_eq!(GateScarabFourType::all().len(), 3);
    }

    #[test]
    fn slugs_are_unique() {
        let v: Vec<&str> = GateScarabFourType::all().iter().map(|t| t.slug()).collect();
        let unique: std::collections::HashSet<&str> = v.iter().copied().collect();
        assert_eq!(v.len(), unique.len());
    }

    #[test]
    fn display_matches_slug() {
        assert_eq!(format!("{}", GateScarabFourType::OType), "SR-03/O-Type");
        assert_eq!(format!("{}", GateScarabFourType::Composite), "SR-03/Composite");
    }

    #[test]
    fn as_markdown_emits_bullet() {
        assert_eq!(GateScarabFourType::OType.as_markdown(), "- **SR-03/O-Type**");
    }

    #[test]
    fn struct_carries_term_pair() {
        let s = GateScarab::new(GateScarabFourType::Composite);
        assert_eq!(s.o_term, Term::OType);
        assert_eq!(s.compound_term, Term::FourType);
    }

    #[test]
    fn struct_serde_round_trip() {
        let s = GateScarab::new(GateScarabFourType::FourType);
        let j = serde_json::to_string(&s).unwrap();
        let back: GateScarab = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }
}
