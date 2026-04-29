//! HY-01 — dispatch (scaffold)
//!
//! Scaffold ring for trios-hybrid (issue #238). Logic migration: TODO.

/// Marker for HY-01 ring scope (dispatch).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Hy01Scope;

impl Hy01Scope {
    pub const RING: &'static str = "HY-01";
    pub const PURPOSE: &'static str = "dispatch";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Hy01Scope::RING, "HY-01");
        assert_eq!(Hy01Scope::PURPOSE, "dispatch");
    }
}
