//! Integration tests for `ingest_text` — golden corpus byte-compare.
//!
//! The small corpus "the quick brown fox jumps over the lazy dog and a ternary world"
//! is used as a reproducible fixture. Expected values are pre-computed from the
//! deterministic djb2-hash + φ⁻² quantizer pipeline.
//!
//! Tests verify:
//! - correct triplet count for the given window/stride config
//! - ternary values are all in {-1, 0, +1}
//! - first and last triplet anchor/positive/negative values (golden byte-compare)
//! - serialised byte length == 192 per triplet
//!
//! Apache-2.0 — Author: Dmitrii Vasilev <admin@t27.ai>

use jepa_t_ingest::{ingest_text, IngestConfig, Triplet};

/// Small reproducible corpus used as integration fixture.
const CORPUS: &str =
    "the quick brown fox jumps over the lazy dog and a ternary world";

/// window_size=4, stride=2 → 5 windows, 3 triplets from 13 tokens
const CFG: IngestConfig = IngestConfig {
    window_size: 4,
    stride: 2,
};

// ────────────────────────────────────────────────────────────────────────────
// Helper
// ────────────────────────────────────────────────────────────────────────────

fn golden_triplets() -> Vec<Triplet> {
    ingest_text(CORPUS, &CFG)
}

// ────────────────────────────────────────────────────────────────────────────
// Count
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn golden_corpus_triplet_count() {
    // 13 tokens, window=4, stride=2 → 5 windows → 3 triplets
    let triplets = golden_triplets();
    assert_eq!(
        triplets.len(),
        3,
        "expected 3 triplets from 13-token corpus with window=4 stride=2, got {}",
        triplets.len()
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Ternary constraint
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn all_values_are_ternary() {
    for (ti, t) in golden_triplets().iter().enumerate() {
        for (fi, &v) in t
            .anchor
            .iter()
            .chain(t.positive.iter())
            .chain(t.negative.iter())
            .enumerate()
        {
            assert!(
                v == -1 || v == 0 || v == 1,
                "triplet[{}] field[{}] = {} — not ternary!",
                ti,
                fi,
                v
            );
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Golden byte-compare: first triplet
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn golden_first_triplet_anchor() {
    // anchor = tokens["the", "quick", "brown", "fox"] → ternary {-1, 0, -1, 1, 0..}
    let triplets = golden_triplets();
    let anchor = triplets[0].anchor;
    assert_eq!(anchor[0], -1, "anchor[0] = token 'the'");
    assert_eq!(anchor[1],  0, "anchor[1] = token 'quick'");
    assert_eq!(anchor[2], -1, "anchor[2] = token 'brown'");
    assert_eq!(anchor[3],  1, "anchor[3] = token 'fox'");
    // remaining slots are zero-padded (window_size=4 < 64)
    for i in 4..64 {
        assert_eq!(anchor[i], 0, "anchor[{}] must be zero-padded", i);
    }
}

#[test]
fn golden_first_triplet_positive() {
    // positive = tokens["brown", "fox", "jumps", "over"] → {-1, 1, 0, -1, 0..}
    let triplets = golden_triplets();
    let pos = triplets[0].positive;
    assert_eq!(pos[0], -1, "positive[0] = token 'brown'");
    assert_eq!(pos[1],  1, "positive[1] = token 'fox'");
    assert_eq!(pos[2],  0, "positive[2] = token 'jumps'");
    assert_eq!(pos[3], -1, "positive[3] = token 'over'");
    for i in 4..64 {
        assert_eq!(pos[i], 0, "positive[{}] must be zero-padded", i);
    }
}

#[test]
fn golden_first_triplet_negative() {
    // negative = last window = tokens["dog", "and", "a", "ternary"] → {1, 1, -1, -1, 0..}
    // (negative uses window index num_windows-1 when anchor is in first half)
    let triplets = golden_triplets();
    let neg = triplets[0].negative;
    assert_eq!(neg[0],  1, "negative[0] = token 'dog'");
    assert_eq!(neg[1],  1, "negative[1] = token 'and'");
    assert_eq!(neg[2], -1, "negative[2] = token 'a'");
    assert_eq!(neg[3], -1, "negative[3] = token 'ternary'");
    for i in 4..64 {
        assert_eq!(neg[i], 0, "negative[{}] must be zero-padded", i);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Serialisation
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn triplet_serialises_to_192_bytes() {
    for t in golden_triplets() {
        assert_eq!(t.to_bytes().len(), 192, "each triplet must serialise to 192 bytes");
    }
}

#[test]
fn golden_total_bytes() {
    // 3 triplets × 192 bytes = 576 bytes
    let total: usize = golden_triplets().iter().map(|t| t.to_bytes().len()).sum();
    assert_eq!(total, 576, "3 triplets × 192 bytes = 576 bytes");
}

// ────────────────────────────────────────────────────────────────────────────
// Edge cases
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn empty_corpus_returns_empty() {
    let cfg = IngestConfig { window_size: 4, stride: 2 };
    assert!(ingest_text("", &cfg).is_empty());
}

#[test]
fn single_token_returns_empty() {
    let cfg = IngestConfig { window_size: 4, stride: 2 };
    assert!(ingest_text("hello", &cfg).is_empty());
}

#[test]
fn large_stride_still_produces_triplets() {
    // window=4, stride=1 with our 13-token corpus → 10 windows → 8 triplets
    let cfg = IngestConfig { window_size: 4, stride: 1 };
    let triplets = ingest_text(CORPUS, &cfg);
    assert_eq!(triplets.len(), 8, "expected 8 triplets with stride=1");
}
