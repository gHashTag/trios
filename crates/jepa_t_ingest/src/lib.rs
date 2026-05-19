//! # jepa_t_ingest
//!
//! Plaintext → ternary-quantized triplet pipeline for JEPA-T training on Trinity silicon.
//!
//! ## Ternary Anchor
//!
//! - Ternary alphabet: {−1, 0, +1}
//! - φ⁻² (Q1.15) = 12533 (0x30F4) — quantization threshold
//! - φ² + φ⁻² = 3
//! - DOI: 10.5281/zenodo.19227877
//!
//! ## Quantizer: Wave-9b RTL Byte-for-Byte Match
//!
//! Matches `phi_prior_quantizer.v` from Wave-9b exactly:
//! - if fp >= +12533 → +1
//! - if fp <= −12533 → −1
//! - else → 0
//!
//! ## License
//!
//! Apache-2.0

/// Ternary quantizer — matches Wave-9b `phi_prior_quantizer.v` byte-for-byte.
///
/// Threshold is φ⁻² in Q1.15 fixed-point = 12533 (0x30F4).
///
/// # Arguments
///
/// * `fp_q15` — signed 16-bit Q1.15 fixed-point input
///
/// # Returns
///
/// * `+1` if `fp_q15 >= 12533`
/// * `-1` if `fp_q15 <= -12533`
/// * `0`  otherwise
///
/// # Examples
///
/// ```
/// use jepa_t_ingest::quantize_phi_prior;
///
/// assert_eq!(quantize_phi_prior(12533), 1);
/// assert_eq!(quantize_phi_prior(-12533), -1);
/// assert_eq!(quantize_phi_prior(12532), 0);
/// assert_eq!(quantize_phi_prior(-12532), 0);
/// assert_eq!(quantize_phi_prior(0), 0);
/// ```
#[inline]
pub fn quantize_phi_prior(fp_q15: i16) -> i8 {
    const THRESHOLD: i16 = 12533; // φ⁻² in Q1.15 = 0x30F4
    if fp_q15 >= THRESHOLD {
        1
    } else if fp_q15 <= -THRESHOLD {
        -1
    } else {
        0
    }
}

/// Configuration for the plaintext ingest pipeline.
///
/// Controls how text is windowed into anchor/positive/negative triplets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestConfig {
    /// Number of tokens per context window (must be <= 64).
    pub window_size: usize,
    /// Stride between successive anchor windows.
    pub stride: usize,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            window_size: 64,
            stride: 32,
        }
    }
}

/// A ternary triplet for JEPA-T contrastive training.
///
/// Each field is a 64-element ternary vector with values in {-1, 0, +1}.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triplet {
    /// Anchor context window (quantized token hashes).
    pub anchor: [i8; 64],
    /// Positive window — overlapping or adjacent to anchor.
    pub positive: [i8; 64],
    /// Negative window — non-overlapping, sampled from elsewhere in the corpus.
    pub negative: [i8; 64],
}

impl Triplet {
    /// Serialize this triplet to raw bytes (192 bytes, i8 → u8 reinterpret).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(192);
        for &v in self.anchor.iter() {
            out.push(v as u8);
        }
        for &v in self.positive.iter() {
            out.push(v as u8);
        }
        for &v in self.negative.iter() {
            out.push(v as u8);
        }
        out
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

/// Hash a token (byte slice) into a Q1.15 signed integer, then ternary-quantize.
///
/// Uses a simple djb2-style accumulation to produce a reproducible i16 value.
fn token_to_q15(token: &[u8]) -> i16 {
    let mut h: u32 = 5381;
    for &b in token {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    // Fold 32-bit hash into i16 range by XOR-folding, preserving sign distribution.
    let folded = ((h >> 16) ^ (h & 0xFFFF)) as u16;
    folded as i16
}

/// Convert a text window (slice of byte-token slices) into a 64-element ternary vector.
///
/// Tokens are hashed to Q1.15, then quantized with `quantize_phi_prior`.
/// If the window has fewer than 64 tokens, the remainder is zero-padded.
fn window_to_ternary(tokens: &[&[u8]]) -> [i8; 64] {
    let mut out = [0i8; 64];
    for (i, tok) in tokens.iter().take(64).enumerate() {
        out[i] = quantize_phi_prior(token_to_q15(tok));
    }
    out
}

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// Stream a plaintext string into a sequence of ternary [`Triplet`]s.
///
/// The algorithm:
/// 1. Tokenise by whitespace into byte slices.
/// 2. Slide an anchor window of `cfg.window_size` tokens with step `cfg.stride`.
/// 3. Positive = next window (`anchor_start + cfg.stride`).
/// 4. Negative = window from the opposite end of the corpus.
/// 5. Each window is mapped to a 64-element ternary vector via [`quantize_phi_prior`].
///
/// At least 3 windows are required to form a triplet (anchor, positive, negative
/// must all be distinct). Returns an empty Vec if the input is too short.
///
/// # Arguments
///
/// * `input` — raw UTF-8 text (any language, any encoding as UTF-8)
/// * `cfg`   — windowing configuration
///
/// # Examples
///
/// ```
/// use jepa_t_ingest::{ingest_text, IngestConfig};
///
/// let cfg = IngestConfig { window_size: 4, stride: 2 };
/// let triplets = ingest_text("the quick brown fox jumps over the lazy dog", &cfg);
/// assert!(!triplets.is_empty());
/// ```
pub fn ingest_text(input: &str, cfg: &IngestConfig) -> Vec<Triplet> {
    let ws = cfg.window_size.min(64).max(1);
    let stride = cfg.stride.max(1);

    // Tokenise by whitespace — collect byte representations.
    let raw_tokens: Vec<&[u8]> = input.split_whitespace().map(str::as_bytes).collect();
    let n = raw_tokens.len();

    if n < ws * 2 {
        // Not enough tokens to form even one meaningful triplet.
        return Vec::new();
    }

    // Build all windows.
    let windows: Vec<[i8; 64]> = (0..)
        .map(|i| i * stride)
        .take_while(|&start| start + ws <= n)
        .map(|start| window_to_ternary(&raw_tokens[start..start + ws]))
        .collect();

    let num_windows = windows.len();
    if num_windows < 3 {
        return Vec::new();
    }

    let mut triplets = Vec::with_capacity(num_windows.saturating_sub(2));

    for i in 0..num_windows - 2 {
        let anchor = windows[i];
        let positive = windows[i + 1];
        // Negative: pick from the farthest window (opposite end from anchor).
        let neg_idx = if i < num_windows / 2 {
            num_windows - 1
        } else {
            0
        };
        let negative = windows[neg_idx];

        triplets.push(Triplet {
            anchor,
            positive,
            negative,
        });
    }

    triplets
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn quantize_zero() {
        assert_eq!(quantize_phi_prior(0), 0);
    }

    #[test]
    fn quantize_positive_boundary() {
        assert_eq!(quantize_phi_prior(12532), 0, "+12532 must be 0");
        assert_eq!(quantize_phi_prior(12533), 1, "+12533 must be +1");
    }

    #[test]
    fn quantize_negative_boundary() {
        assert_eq!(quantize_phi_prior(-12532), 0, "-12532 must be 0");
        assert_eq!(quantize_phi_prior(-12533), -1, "-12533 must be -1");
    }

    #[test]
    fn quantize_max_values() {
        assert_eq!(quantize_phi_prior(i16::MAX), 1);
        assert_eq!(quantize_phi_prior(i16::MIN), -1);
    }

    #[test]
    fn ingest_empty_returns_empty() {
        let cfg = IngestConfig::default();
        assert!(ingest_text("", &cfg).is_empty());
    }

    #[test]
    fn ingest_short_returns_empty() {
        let cfg = IngestConfig { window_size: 64, stride: 32 };
        assert!(ingest_text("hello world", &cfg).is_empty());
    }

    #[test]
    fn ingest_produces_valid_ternary() {
        let cfg = IngestConfig { window_size: 4, stride: 2 };
        let corpus = "a b c d e f g h i j k l m n o p";
        let triplets = ingest_text(corpus, &cfg);
        assert!(!triplets.is_empty());
        for t in &triplets {
            for &v in t.anchor.iter().chain(t.positive.iter()).chain(t.negative.iter()) {
                assert!(v == -1 || v == 0 || v == 1, "non-ternary value: {}", v);
            }
        }
    }

    #[test]
    fn triplet_to_bytes_length() {
        let t = Triplet {
            anchor: [1i8; 64],
            positive: [0i8; 64],
            negative: [-1i8; 64],
        };
        assert_eq!(t.to_bytes().len(), 192);
    }
}
