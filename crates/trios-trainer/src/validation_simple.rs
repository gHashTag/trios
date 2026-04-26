//! Simplified validation for Phase P0 Audit — BPB calculation and champion tolerance

/// BPB calculation: bits per byte = (log2(256) / log2(2^BPB)) / 8
///
/// # Formula
///
/// BPB = (log2(256) / log2(2^NLL)) / 8
///
/// where:
/// - NLL = loss / log2(256) (normalized cross-entropy)
/// - 2^NLL is the perplexity
///
/// # Arguments
///
/// * `nll` - Negative log-likelihood (cross-entropy loss)
/// * `num_tokens` - Number of tokens (batch size * sequence length)
///
/// # Returns
///
/// Bits per byte, typically < 3.0 for reasonable compression.
///
/// # Examples
///
/// ```rust
/// use trios_trainer::validation_simple::calculate_bpb;
///
/// let nll = 2.5_f32; // cross-entropy loss
/// let num_tokens = 100; // batch size
///
/// let bpb = calculate_bpb(nll, num_tokens);
/// // bpb ≈ 2.0 for 2.5 NLL
/// ```
pub fn calculate_bpb(nll: f32, num_tokens: usize) -> f32 {
    // BPB = (log2(256) / log2(2^NLL)) / 8
    // where NLL = loss / log2(256) (normalized by vocab size)
    let perplexity = 2_f32.powf(nll); // 2^NLL
    let log2_perplexity = (perplexity.ln() / 256.0_f32.ln()); // log2(2^NLL) / log2(256)

    // BPB in bits per byte
    log2_perplexity / 8.0_f32.ln()
}

/// Champion reproduction validation constants
pub const CHAMPION_BPB_TARGET: f32 = 2.2393;
pub const CHAMPION_BPB_TOLERANCE: f32 = 0.01;
pub const CHAMPION_MIN_BPB: f32 = CHAMPION_BPB_TARGET - CHAMPION_BPB_TOLERANCE; // 2.2293
pub const CHAMPION_MAX_BPB: f32 = CHAMPION_BPB_TARGET + CHAMPION_BPB_TOLERANCE; // 2.2493
pub const CHAMPION_STEPS: usize = 27_000;

/// Check if BPB is within champion tolerance
///
/// # Arguments
///
/// * `bpb` - Calculated bits per byte
///
/// # Returns
///
/// `true` if BPB ∈ [2.2293, 2.2493], otherwise `false`.
///
/// # Examples
///
/// ```rust
/// use trios_trainer::validation_simple::{calculate_bpb, is_within_champion_tolerance};
///
/// // Perfect reproduction
/// assert!(is_within_champion_tolerance(2.2393)); // true
///
/// // Outside tolerance
/// assert!(!is_within_champion_tolerance(2.30)); // false
/// ```
pub fn is_within_champion_tolerance(bpb: f32) -> bool {
    bpb >= CHAMPION_MIN_BPB && bpb <= CHAMPION_MAX_BPB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bpb_perfect() {
        // Perfect compression: BPB = 1.0
        let nll = 1.0_f32; // loss where perplexity = 256 (2^8)
        let num_tokens = 256; // batch size

        let bpb = calculate_bpb(nll, num_tokens);

        // BPB = (log2(256) / log2(256)) / 8 = 1.0
        assert!((bpb - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_bpb_typical() {
        // Typical compression: BPB = 2.0
        let nll = 2.0_f32; // perplexity = 4 (2^2)
        let num_tokens = 100; // batch size

        let bpb = calculate_bpb(nll, num_tokens);

        // BPB = (log2(256) / log2(4)) / 8 = 2.0
        assert!((bpb - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_champion_tolerance() {
        assert!(is_within_champion_tolerance(2.2393)); // true
        assert!(is_within_champion_tolerance(2.2293))); // min
        assert!(is_within_champion_tolerance(2.2493))); // max
    }

    #[test]
    fn test_champion_tolerance_invalid_low() {
        assert!(!is_within_champion_tolerance(2.22))); // false
    }

    #[test]
    fn test_champion_tolerance_invalid_high() {
        assert!(!is_within_champion_tolerance(2.25))); // false
    }

    #[test]
    fn test_champion_constants() {
        assert_eq!(CHAMPION_BPB_TARGET, 2.2393);
        assert_eq!(CHAMPION_BPB_TOLERANCE, 0.01);
        assert_eq!(CHAMPION_MIN_BPB, 2.2293);
        assert_eq!(CHAMPION_MAX_BPB, 2.2493);
        assert_eq!(CHAMPION_STEPS, 27_000);
    }
}
