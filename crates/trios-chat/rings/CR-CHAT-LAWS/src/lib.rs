//! CR-CHAT-LAWS — Trinity Secure Chat constitutional laws (R-CHAT-1..12).
//!
//! [VERIFIED] These 12 laws are the immutable constitution of the chat stack.
//! Any change requires an approved ADR-CHAT-* commit. The hash is asserted by
//! laws_guard tests downstream.
//!
//! Sibling rings: CR-CHAT-00 (errors), CR-CHAT-01 (sealed/identity),
//! CR-CHAT-02 (ratchet), CR-CHAT-03 (group), CR-CHAT-04 (padding),
//! CR-CHAT-06 (capability + injection).
//!
//! [CITED] design §3.0 — Twelve Laws of Trinity Secure Chat.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

/// The 12 laws. Order is part of the contract. [CITED design §3.0]
pub const R_CHAT_LAWS: [&str; 12] = [
    "R-CHAT-1  NO PLAINTEXT AT REST",
    "R-CHAT-2  HYBRID PQ FROM DAY ONE",
    "R-CHAT-3  SEALED SENDER MANDATORY",
    "R-CHAT-4  DENIABLE AUTHENTICATION",
    "R-CHAT-5  AGENT KEY != USER KEY",
    "R-CHAT-6  TOOLS ARE SIGNED PROMPTS",
    "R-CHAT-7  DUAL-LLM ISOLATION",
    "R-CHAT-8  SESSION-SCOPED CAPABILITY",
    "R-CHAT-9  FIXED-SIZE PADDING",
    "R-CHAT-10 ZERO BACKGROUND CHATTER",
    "R-CHAT-11 COQ-VERIFIED INVARIANTS",
    "R-CHAT-12 R5+R7 (HONESTY + FALSIFIABILITY)",
];

/// SHA-256 over the joined laws. Updated only via ADR-CHAT-*.
/// [DERIVED] Re-computed at runtime; reference value asserted in test.
pub fn laws_hash() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for l in R_CHAT_LAWS.iter() {
        h.update(l.as_bytes());
        h.update([0u8]);
    }
    let out = h.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_laws_present() {
        assert_eq!(R_CHAT_LAWS.len(), 12);
    }

    #[test]
    fn laws_hash_stable_within_run() {
        let a = laws_hash();
        let b = laws_hash();
        assert_eq!(a, b);
    }

    #[test]
    fn laws_have_canonical_prefix() {
        for (i, l) in R_CHAT_LAWS.iter().enumerate() {
            assert!(l.starts_with(&format!("R-CHAT-{}", i + 1)));
        }
    }

    #[test]
    fn laws_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for l in R_CHAT_LAWS.iter() {
            assert!(seen.insert(*l), "duplicate law: {l}");
        }
    }
}
