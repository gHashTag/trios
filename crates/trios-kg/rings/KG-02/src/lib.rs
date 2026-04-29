//! KG-02 — index (scaffold)
//!
//! Scaffold ring for trios-kg (issue #238). Logic migration: TODO.

/// Marker for KG-02 ring scope (index).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Kg02Scope;

impl Kg02Scope {
    pub const RING: &'static str = "KG-02";
    pub const PURPOSE: &'static str = "index";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Kg02Scope::RING, "KG-02");
        assert_eq!(Kg02Scope::PURPOSE, "index");
    }
}
