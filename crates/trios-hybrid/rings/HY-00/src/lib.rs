//! HY-00 — runtime (scaffold)
//!
//! Scaffold ring for trios-hybrid (issue #238). Logic migration: TODO.

/// Marker for HY-00 ring scope (runtime).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Hy00Scope;

impl Hy00Scope {
    pub const RING: &'static str = "HY-00";
    pub const PURPOSE: &'static str = "runtime";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Hy00Scope::RING, "HY-00");
        assert_eq!(Hy00Scope::PURPOSE, "runtime");
    }
}
