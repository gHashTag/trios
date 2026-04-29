//! GB-00 — branches
//!
//! Ring scaffold for `trios-gb`. Logic lives in the parent crate's
//! `src/` until migrated. This ring is additive scaffolding under L-ARCH-001
//! to satisfy the ring isolation contract required by issue #238.

/// Marker placeholder so the ring compiles cleanly.
pub fn ring_id() -> &'static str {
    "GB-00"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_id_matches() {
        assert_eq!(ring_id(), "GB-00");
    }
}
