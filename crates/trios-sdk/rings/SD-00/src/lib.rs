//! SD-00 — api (scaffold)
//!
//! Scaffold ring for trios-sdk (issue #238). Logic migration: TODO.

/// Marker for SD-00 ring scope (api).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sd00Scope;

impl Sd00Scope {
    pub const RING: &'static str = "SD-00";
    pub const PURPOSE: &'static str = "api";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Sd00Scope::RING, "SD-00");
        assert_eq!(Sd00Scope::PURPOSE, "api");
    }
}
