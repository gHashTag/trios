//! SD-02 — auth (scaffold)
//!
//! Scaffold ring for trios-sdk (issue #238). Logic migration: TODO.

/// Marker for SD-02 ring scope (auth).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sd02Scope;

impl Sd02Scope {
    pub const RING: &'static str = "SD-02";
    pub const PURPOSE: &'static str = "auth";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Sd02Scope::RING, "SD-02");
        assert_eq!(Sd02Scope::PURPOSE, "auth");
    }
}
