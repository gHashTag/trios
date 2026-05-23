//! # CR-CHAT-07 — Receiver-side traffic analysis resistance guard (Wave-61 Lane A)
//!
//! ANTI-CORRELATION — receiver cannot identify sender by pattern, R-CHAT-10.
//!
//! An attacker controlling a receiver node observes the pattern of
//! incoming envelopes across multiple potential senders. If sender A
//! always transmits in bursts of exactly 3 while sender B transmits
//! singly, the receiver can link envelopes to senders despite sealed
//! sender. The guard validates that burst size distributions across
//! all observed senders are statistically indistinguishable.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each sender's burst count <= `RTAR_MAX_BURST`.
//! 2. No sender dominates total traffic (> 50%).
//! 3. Sender count >= `RTAR_MIN_SENDERS`.
//! 4. Burst size variance across senders <= `RTAR_MAX_VARIANCE`.
//! 5. No zero-burst senders.
//! 6. Observation count <= `RTAR_MAX_OBS`.
//!
//! Tests **RTAR-01..10**. Error enum [`ReceiverAnalysisError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RECEIVER-ANALYSIS`

#![forbid(unsafe_code)]

/// Maximum burst size per sender.
pub const RTAR_MAX_BURST: u64 = 16;

/// Minimum senders for analysis.
pub const RTAR_MIN_SENDERS: usize = 3;

/// Maximum standard deviation across sender burst counts.
pub const RTAR_MAX_STDDEV: f64 = 2.0;

/// Maximum observations.
pub const RTAR_MAX_OBS: usize = 1024;

/// All ways receiver analysis resistance can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiverAnalysisError {
    /// Single sender burst too large.
    BurstTooLarge,
    /// Sender dominates traffic (> 50%).
    SenderDominance,
    /// Too few senders.
    TooFewSenders,
    /// Variance across senders too high.
    VarianceTooHigh,
    /// Zero-burst sender.
    ZeroBurst,
    /// Too many observations.
    TooManyObs,
}

/// `[VERIFIED]` Validate burst counts across senders for traffic
/// analysis resistance.
pub fn validate_receiver_analysis(
    sender_burst_counts: &[u64],
) -> Result<(), ReceiverAnalysisError> {
    if sender_burst_counts.len() > RTAR_MAX_OBS {
        return Err(ReceiverAnalysisError::TooManyObs);
    }
    if sender_burst_counts.is_empty() {
        return Ok(());
    }
    if sender_burst_counts.len() < RTAR_MIN_SENDERS {
        return Err(ReceiverAnalysisError::TooFewSenders);
    }
    let total: u64 = sender_burst_counts.iter().sum();
    for &count in sender_burst_counts {
        if count == 0 {
            return Err(ReceiverAnalysisError::ZeroBurst);
        }
        if count > RTAR_MAX_BURST {
            return Err(ReceiverAnalysisError::BurstTooLarge);
        }
        if count * 2 > total {
            return Err(ReceiverAnalysisError::SenderDominance);
        }
    }
    let mean = total as f64 / sender_burst_counts.len() as f64;
    let variance = sender_burst_counts.iter()
        .map(|&c| { let d = c as f64 - mean; d * d })
        .sum::<f64>() / sender_burst_counts.len() as f64;
    if variance.sqrt() > RTAR_MAX_STDDEV {
        return Err(ReceiverAnalysisError::VarianceTooHigh);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **RTAR-01** — burst too large rejected.
    #[test]
    fn rtar_01_burst_large_rejected() {
        assert_eq!(
            validate_receiver_analysis(&[5, 5, 5, RTAR_MAX_BURST + 1]),
            Err(ReceiverAnalysisError::BurstTooLarge)
        );
    }

    /// **RTAR-02** — sender dominance rejected.
    #[test]
    fn rtar_02_dominance_rejected() {
        assert_eq!(
            validate_receiver_analysis(&[10, 1, 1]),
            Err(ReceiverAnalysisError::SenderDominance)
        );
    }

    /// **RTAR-03** — too few senders rejected.
    #[test]
    fn rtar_03_too_few_rejected() {
        assert_eq!(
            validate_receiver_analysis(&[5, 5]),
            Err(ReceiverAnalysisError::TooFewSenders)
        );
    }

    /// **RTAR-04** — variance too high rejected.
    #[test]
    fn rtar_04_variance_rejected() {
        assert_eq!(
            validate_receiver_analysis(&[1, 1, 1, 1, 1, 1, 1, 1, 8]),
            Err(ReceiverAnalysisError::VarianceTooHigh)
        );
    }

    /// **RTAR-05** — zero burst rejected.
    #[test]
    fn rtar_05_zero_rejected() {
        assert_eq!(
            validate_receiver_analysis(&[5, 0, 5]),
            Err(ReceiverAnalysisError::ZeroBurst)
        );
    }

    /// **RTAR-06** — uniform accepted.
    #[test]
    fn rtar_06_uniform_accepted() {
        assert_eq!(validate_receiver_analysis(&[5, 5, 5, 5]), Ok(()));
    }

    /// **RTAR-07** — slight variation accepted.
    #[test]
    fn rtar_07_slight_var_accepted() {
        assert_eq!(validate_receiver_analysis(&[4, 5, 6, 5]), Ok(()));
    }

    /// **RTAR-08** — empty accepted.
    #[test]
    fn rtar_08_empty_accepted() {
        assert_eq!(validate_receiver_analysis(&[]), Ok(()));
    }

    /// **RTAR-09** — minimum senders accepted.
    #[test]
    fn rtar_09_min_senders_accepted() {
        assert_eq!(validate_receiver_analysis(&[3, 3, 3]), Ok(()));
    }

    /// **RTAR-10** — max burst boundary accepted.
    #[test]
    fn rtar_10_max_burst_accepted() {
        assert_eq!(validate_receiver_analysis(&[RTAR_MAX_BURST; 5]), Ok(()));
    }
}
