//! BR-02 — sse (scaffold)
//!
//! Scaffold ring for trios-bridge (issue #238). Logic migration: TODO.

/// Marker for BR-02 ring scope (sse).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Br02Scope;

impl Br02Scope {
    pub const RING: &'static str = "BR-02";
    pub const PURPOSE: &'static str = "sse";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Br02Scope::RING, "BR-02");
        assert_eq!(Br02Scope::PURPOSE, "sse");
    }
}
