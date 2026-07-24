//! BW-00 — core types for `trios-browser`.
//!
//! Scaffold ring (Wave 0). Pure data + serde; no I/O, no async.
//! Business logic to be ported here from the TS backend during migration.

use serde::{Deserialize, Serialize};

/// Placeholder marker type so the ring compiles as a real workspace member.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Placeholder;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholder_default() {
        let _ = Placeholder::default();
    }
}
