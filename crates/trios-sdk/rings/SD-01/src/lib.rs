//! SD-01 — client (scaffold)
//!
//! Scaffold ring for trios-sdk (issue #238). Logic migration: TODO.

/// Marker for SD-01 ring scope (client).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sd01Scope;

impl Sd01Scope {
    pub const RING: &'static str = "SD-01";
    pub const PURPOSE: &'static str = "client";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Sd01Scope::RING, "SD-01");
        assert_eq!(Sd01Scope::PURPOSE, "client");
    }
}
