//! R-CHAT laws guard — fails CI if the constitutional laws are altered
//! outside of an approved ADR-CHAT-* commit.

use trios_chat::r_chat::{laws_hash, R_CHAT_LAWS};

const EXPECTED_COUNT: usize = 12;

#[test]
fn law_count_locked() {
    assert_eq!(R_CHAT_LAWS.len(), EXPECTED_COUNT, "R-CHAT law count changed — update via ADR");
}

#[test]
fn law_titles_locked() {
    let expected_prefixes = [
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
    for (i, p) in expected_prefixes.iter().enumerate() {
        assert_eq!(R_CHAT_LAWS[i], *p, "R-CHAT-{} drifted from ADR-locked text", i + 1);
    }
}

#[test]
fn laws_hash_nonzero_and_stable() {
    let h1 = laws_hash();
    let h2 = laws_hash();
    assert_eq!(h1, h2);
    assert!(h1.iter().any(|b| *b != 0));
}
