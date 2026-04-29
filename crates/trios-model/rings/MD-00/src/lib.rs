//! MD-00 — schema (scaffold)
//!
//! Scaffold ring for trios-model (issue #238). Logic migration: TODO.

/// Marker for MD-00 ring scope (schema).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Md00Scope;

impl Md00Scope {
    pub const RING: &'static str = "MD-00";
    pub const PURPOSE: &'static str = "schema";

    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_metadata() {
        assert_eq!(Md00Scope::RING, "MD-00");
        assert_eq!(Md00Scope::PURPOSE, "schema");
    }
}
