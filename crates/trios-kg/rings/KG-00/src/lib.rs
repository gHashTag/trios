//! KG-00 — graph (scaffold)
//!
//! Scaffold ring for trios-kg (issue #238). Logic migration: TODO.

/// Marker for KG-00 ring scope (graph).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Kg00Scope;

impl Kg00Scope {
    pub const RING: &'static str = "KG-00";
    pub const PURPOSE: &'static str = "graph";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Kg00Scope::RING, "KG-00");
        assert_eq!(Kg00Scope::PURPOSE, "graph");
    }
}
