//! DT-02 — sync (scaffold)
//!
//! Scaffold ring for trios-data (issue #238). Logic migration: TODO.

/// Marker for DT-02 ring scope (sync).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Dt02Scope;

impl Dt02Scope {
    pub const RING: &'static str = "DT-02";
    pub const PURPOSE: &'static str = "sync";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Dt02Scope::RING, "DT-02");
        assert_eq!(Dt02Scope::PURPOSE, "sync");
    }
}
