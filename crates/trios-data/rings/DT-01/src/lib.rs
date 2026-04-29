//! DT-01 — query (scaffold)
//!
//! Scaffold ring for trios-data (issue #238). Logic migration: TODO.

/// Marker for DT-01 ring scope (query).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Dt01Scope;

impl Dt01Scope {
    pub const RING: &'static str = "DT-01";
    pub const PURPOSE: &'static str = "query";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Dt01Scope::RING, "DT-01");
        assert_eq!(Dt01Scope::PURPOSE, "query");
    }
}
