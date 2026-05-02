//! SR-04 scarab-gate-five ring.
//!
//! Combines O-Type + Five-Type for gate-five-level agents.
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
pub enum GateScarabFiveType {
    /// O-Type carrier (origin / monad term).
    OType,
    /// Five-Type carrier.
    FiveType,
    /// Composite — both O-Type and Five-Type are bound.
    Composite,
}

impl GateScarabFiveType {
    /// Stable string slug.
    pub fn slug(self) -> &'static str {
        match self {
            GateScarabFiveType::OType => "SR-04/O-Type",
            GateScarabFiveType::FiveType => "SR-04/Five-Type",
            GateScarabFiveType::Composite => "SR-04/Composite",
        }
    }

    /// Markdown bullet representation.
    pub fn as_markdown(self) -> String {
        format!("- **{}**", self.slug())
    }

    /// All variants in declaration order.
    pub fn all() -> [GateScarabFiveType; 3] {
        [
            GateScarabFiveType::OType,
            GateScarabFiveType::FiveType,
            GateScarabFiveType::Composite,
        ]
    }
}

impl fmt::Display for GateScarabFiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// GateScarab — gate-five-level scarab. Carries a `GateScarabFiveType` and the
/// underlying SR-00 `Term` pair (O-Type, FiveType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GateScarab {
    /// Variant tag.
    pub kind: GateScarabFiveType,
    /// O-Type term (always bound).
    pub o_term: Term,
    /// Five-Type term (always bound).
    pub compound_term: Term,
}

impl GateScarab {
    /// Build a fresh gate-five-level scarab.
    pub fn new(kind: GateScarabFiveType) -> Self {
        Self {
            kind,
            o_term: Term::OType,
            compound_term: Term::FiveType,
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
        assert_eq!(GateScarabFiveType::all().len(), 3);
    }

    #[test]
    fn slugs_are_unique() {
        let v: Vec<&str> = GateScarabFiveType::all().iter().map(|t| t.slug()).collect();
        let unique: std::collections::HashSet<&str> = v.iter().copied().collect();
        assert_eq!(v.len(), unique.len());
    }

    #[test]
    fn display_matches_slug() {
        assert_eq!(format!("{}", GateScarabFiveType::OType), "SR-04/O-Type");
        assert_eq!(format!("{}", GateScarabFiveType::Composite), "SR-04/Composite");
    }

    #[test]
    fn as_markdown_emits_bullet() {
        assert_eq!(GateScarabFiveType::OType.as_markdown(), "- **SR-04/O-Type**");
    }

    #[test]
    fn struct_carries_term_pair() {
        let s = GateScarab::new(GateScarabFiveType::Composite);
        assert_eq!(s.o_term, Term::OType);
        assert_eq!(s.compound_term, Term::FiveType);
    }

    #[test]
    fn struct_serde_round_trip() {
        let s = GateScarab::new(GateScarabFiveType::FiveType);
        let j = serde_json::to_string(&s).unwrap();
        let back: GateScarab = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }
}
