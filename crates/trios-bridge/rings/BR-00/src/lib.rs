//! BR-00 — transport (scaffold)
//!
//! Scaffold ring for trios-bridge (issue #238). Logic migration: TODO.

/// Marker for BR-00 ring scope (transport).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Br00Scope;

impl Br00Scope {
    pub const RING: &'static str = "BR-00";
    pub const PURPOSE: &'static str = "transport";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Br00Scope::RING, "BR-00");
        assert_eq!(Br00Scope::PURPOSE, "transport");
    }
}
