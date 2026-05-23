//! # CR-CHAT-07 — Traffic shaping burst uniformity guard (Wave-65 Lane B)
//!
//! ANTI-CORRELATION — shaped bursts must be uniform, R-CHAT-7.
//!
//! Traffic shaping hides message sizes by batching messages into uniform
//! bursts. If bursts are not uniform, an observer can:
//!
//! * **Distinguish real from padding** — non-uniform burst sizes leak
//!   which bursts carry real traffic.
//! * **Fingerprint users** — burst timing patterns are unique per user.
//! * **Count messages** — varying burst sizes reveal message count.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All burst sizes in a batch are equal.
//! 2. Burst size >= `TSBU_MIN_BURST`.
//! 3. Burst size <= `TSBU_MAX_BURST`.
//! 4. Number of bursts >= `TSBU_MIN_COUNT`.
//! 5. Number of bursts <= `TSBU_MAX_COUNT`.
//! 6. Inter-burst intervals are equal (timing uniformity).
//!
//! Tests **TSBU-01..10**. Error enum [`BurstUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BURST-UNIFORMITY`

#![forbid(unsafe_code)]

/// Minimum burst size (bytes).
pub const TSBU_MIN_BURST: usize = 64;

/// Maximum burst size (bytes).
pub const TSBU_MAX_BURST: usize = 65536;

/// Minimum number of bursts in a batch.
pub const TSBU_MIN_COUNT: usize = 2;

/// Maximum number of bursts in a batch.
pub const TSBU_MAX_COUNT: usize = 256;

/// All ways burst uniformity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BurstUniformityError {
    /// Burst sizes not uniform.
    NonUniformSize,
    /// Burst size too small.
    BurstTooSmall,
    /// Burst size too large.
    BurstTooLarge,
    /// Too few bursts.
    TooFewBursts,
    /// Too many bursts.
    TooManyBursts,
    /// Inter-burst intervals not uniform.
    NonUniformInterval,
}

/// A shaped burst.
#[derive(Debug, Clone)]
pub struct ShapedBurst {
    /// Burst size in bytes.
    pub size: usize,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

/// `[VERIFIED]` Validate that a batch of shaped bursts is uniform.
pub fn validate_burst_uniformity(
    bursts: &[ShapedBurst],
) -> Result<(), BurstUniformityError> {
    if bursts.len() < TSBU_MIN_COUNT {
        return Err(BurstUniformityError::TooFewBursts);
    }
    if bursts.len() > TSBU_MAX_COUNT {
        return Err(BurstUniformityError::TooManyBursts);
    }
    let first_size = bursts[0].size;
    if first_size < TSBU_MIN_BURST {
        return Err(BurstUniformityError::BurstTooSmall);
    }
    if first_size > TSBU_MAX_BURST {
        return Err(BurstUniformityError::BurstTooLarge);
    }
    for b in &bursts[1..] {
        if b.size != first_size {
            return Err(BurstUniformityError::NonUniformSize);
        }
        if b.size < TSBU_MIN_BURST {
            return Err(BurstUniformityError::BurstTooSmall);
        }
        if b.size > TSBU_MAX_BURST {
            return Err(BurstUniformityError::BurstTooLarge);
        }
    }
    if bursts.len() >= 3 {
        let interval = bursts[1].timestamp_ms as i64 - bursts[0].timestamp_ms as i64;
        for w in bursts.windows(2) {
            let gap = w[1].timestamp_ms as i64 - w[0].timestamp_ms as i64;
            if gap != interval {
                return Err(BurstUniformityError::NonUniformInterval);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burst(ts: u64, size: usize) -> ShapedBurst {
        ShapedBurst { size, timestamp_ms: ts }
    }

    fn uniform_batch() -> Vec<ShapedBurst> {
        vec![
            burst(1000, 1024),
            burst(2000, 1024),
            burst(3000, 1024),
            burst(4000, 1024),
        ]
    }

    /// **TSBU-01** — non-uniform size rejected.
    #[test]
    fn tsbu_01_non_uniform_size_rejected() {
        let batch = vec![burst(1000, 1024), burst(2000, 2048), burst(3000, 1024)];
        assert_eq!(
            validate_burst_uniformity(&batch),
            Err(BurstUniformityError::NonUniformSize)
        );
    }

    /// **TSBU-02** — burst too small rejected.
    #[test]
    fn tsbu_02_too_small_rejected() {
        let batch = vec![burst(1000, 32), burst(2000, 32)];
        assert_eq!(
            validate_burst_uniformity(&batch),
            Err(BurstUniformityError::BurstTooSmall)
        );
    }

    /// **TSBU-03** — burst too large rejected.
    #[test]
    fn tsbu_03_too_large_rejected() {
        let batch = vec![burst(1000, TSBU_MAX_BURST + 1), burst(2000, TSBU_MAX_BURST + 1)];
        assert_eq!(
            validate_burst_uniformity(&batch),
            Err(BurstUniformityError::BurstTooLarge)
        );
    }

    /// **TSBU-04** — too few bursts rejected.
    #[test]
    fn tsbu_04_too_few_rejected() {
        let batch = vec![burst(1000, 1024)];
        assert_eq!(
            validate_burst_uniformity(&batch),
            Err(BurstUniformityError::TooFewBursts)
        );
    }

    /// **TSBU-05** — too many bursts rejected.
    #[test]
    fn tsbu_05_too_many_rejected() {
        let batch: Vec<ShapedBurst> = (0..=TSBU_MAX_COUNT)
            .map(|i| burst(i as u64 * 1000, 1024))
            .collect();
        assert_eq!(
            validate_burst_uniformity(&batch),
            Err(BurstUniformityError::TooManyBursts)
        );
    }

    /// **TSBU-06** — non-uniform interval rejected.
    #[test]
    fn tsbu_06_non_uniform_interval_rejected() {
        let batch = vec![burst(1000, 1024), burst(2000, 1024), burst(4000, 1024)];
        assert_eq!(
            validate_burst_uniformity(&batch),
            Err(BurstUniformityError::NonUniformInterval)
        );
    }

    /// **TSBU-07** — uniform batch accepted.
    #[test]
    fn tsbu_07_uniform_accepted() {
        assert_eq!(validate_burst_uniformity(&uniform_batch()), Ok(()));
    }

    /// **TSBU-08** — two bursts accepted (interval not checked).
    #[test]
    fn tsbu_08_two_bursts_accepted() {
        let batch = vec![burst(1000, 512), burst(9999, 512)];
        assert_eq!(validate_burst_uniformity(&batch), Ok(()));
    }

    /// **TSBU-09** — min burst size accepted.
    #[test]
    fn tsbu_09_min_burst_accepted() {
        let batch = vec![burst(1000, TSBU_MIN_BURST), burst(2000, TSBU_MIN_BURST)];
        assert_eq!(validate_burst_uniformity(&batch), Ok(()));
    }

    /// **TSBU-10** — max burst size accepted.
    #[test]
    fn tsbu_10_max_burst_accepted() {
        let batch = vec![burst(1000, TSBU_MAX_BURST), burst(2000, TSBU_MAX_BURST)];
        assert_eq!(validate_burst_uniformity(&batch), Ok(()));
    }
}
