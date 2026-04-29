//! DT-00 — store (scaffold)
//!
//! Scaffold ring for trios-data (issue #238). Logic migration: TODO.

/// Marker for DT-00 ring scope (store).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Dt00Scope;

impl Dt00Scope {
    pub const RING: &'static str = "DT-00";
    pub const PURPOSE: &'static str = "store";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Dt00Scope::RING, "DT-00");
        assert_eq!(Dt00Scope::PURPOSE, "store");
    }
}
