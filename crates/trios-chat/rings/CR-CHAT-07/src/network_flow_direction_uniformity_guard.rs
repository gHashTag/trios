//! # CR-CHAT-07 — Network flow direction uniformity guard (Wave-95 Lane B)
//!
//! ANTI-CORRELATION — inbound/outbound patterns must be uniform,
//! R-CHAT-10.
//!
//! In a mesh network, each node both sends and receives. If traffic
//! patterns are asymmetric:
//!
//! * **Role detection** — nodes with mostly outbound traffic are
//!   identified as producers; mostly inbound as consumers, revealing
//!   the network topology.
//! * **Client-server fingerprint** — a node that only sends and never
//!   receives (or vice versa) is clearly a client, not a peer.
//! * **Behavioral analysis** — the ratio of inbound to outbound
//!   messages creates a unique fingerprint per node.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Inbound count >= `NFDU_MIN_PER_DIRECTION`.
//! 2. Outbound count >= `NFDU_MIN_PER_DIRECTION`.
//! 3. Direction ratio must be within `NFDU_MAX_RATIO`.
//! 4. Total observations <= `NFDU_MAX_OBSERVATIONS`.
//! 5. Timestamps must be increasing.
//! 6. Direction must be valid (inbound or outbound).
//!
//! Tests **NFDU-01..10**. Error enum [`FlowError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FLOW-UNIFORM`

#![forbid(unsafe_code)]

/// Minimum observations per direction.
pub const NFDU_MIN_PER_DIRECTION: usize = 10;

/// Maximum ratio between directions (numerator).
/// Ratio = max(in,out) / min(in,out) must be <= NFDU_MAX_RATIO_NUM/NFDU_MAX_RATIO_DEN.
pub const NFDU_MAX_RATIO_NUM: usize = 3;
pub const NFDU_MAX_RATIO_DEN: usize = 1;

/// Maximum observations.
pub const NFDU_MAX_OBSERVATIONS: usize = 65536;

/// Flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    /// Inbound message.
    Inbound,
    /// Outbound message.
    Outbound,
}

/// A flow observation.
#[derive(Debug, Clone)]
pub struct FlowObservation {
    /// Direction.
    pub direction: FlowDirection,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

/// All ways flow validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FlowError {
    /// Too few inbound.
    TooFewInbound { count: usize, min: usize },
    /// Too few outbound.
    TooFewOutbound { count: usize, min: usize },
    /// Ratio exceeded.
    RatioExceeded { inbound: usize, outbound: usize },
    /// Too many observations.
    TooManyObservations,
    /// Timestamps not increasing.
    TimestampsNotIncreasing,
}

/// `[VERIFIED]` Validate network flow direction uniformity.
pub fn validate_flow_uniformity(
    observations: &[FlowObservation],
) -> Result<(), FlowError> {
    if observations.len() > NFDU_MAX_OBSERVATIONS {
        return Err(FlowError::TooManyObservations);
    }
    for i in 1..observations.len() {
        if observations[i].timestamp_ms <= observations[i - 1].timestamp_ms {
            return Err(FlowError::TimestampsNotIncreasing);
        }
    }
    if observations.is_empty() {
        return Ok(());
    }
    let inbound = observations.iter().filter(|o| o.direction == FlowDirection::Inbound).count();
    let outbound = observations.len() - inbound;
    if inbound < NFDU_MIN_PER_DIRECTION {
        return Err(FlowError::TooFewInbound { count: inbound, min: NFDU_MIN_PER_DIRECTION });
    }
    if outbound < NFDU_MIN_PER_DIRECTION {
        return Err(FlowError::TooFewOutbound { count: outbound, min: NFDU_MIN_PER_DIRECTION });
    }
    let (larger, smaller) = if inbound > outbound { (inbound, outbound) } else { (outbound, inbound) };
    let ratio = (larger * NFDU_MAX_RATIO_DEN + smaller - 1) / smaller;
    if ratio > NFDU_MAX_RATIO_NUM {
        return Err(FlowError::RatioExceeded { inbound, outbound });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(dir: FlowDirection, ts: u64) -> FlowObservation {
        FlowObservation { direction: dir, timestamp_ms: ts }
    }

    fn balanced_flow() -> Vec<FlowObservation> {
        let mut v = Vec::new();
        for i in 0..15 {
            v.push(obs(FlowDirection::Inbound, i * 2 as u64));
            v.push(obs(FlowDirection::Outbound, i * 2 as u64 + 1));
        }
        v
    }

    /// **NFDU-01** — too few inbound rejected.
    #[test]
    fn nfdu_01_too_few_inbound_rejected() {
        let mut obs_vec: Vec<FlowObservation> = (0..15)
            .map(|i| obs(FlowDirection::Outbound, i as u64 * 100))
            .collect();
        obs_vec.push(obs(FlowDirection::Inbound, 1500));
        assert_eq!(
            validate_flow_uniformity(&obs_vec),
            Err(FlowError::TooFewInbound { count: 1, min: 10 })
        );
    }

    /// **NFDU-02** — too few outbound rejected.
    #[test]
    fn nfdu_02_too_few_outbound_rejected() {
        let mut obs_vec: Vec<FlowObservation> = (0..15)
            .map(|i| obs(FlowDirection::Inbound, i as u64 * 100))
            .collect();
        obs_vec.push(obs(FlowDirection::Outbound, 1500));
        assert_eq!(
            validate_flow_uniformity(&obs_vec),
            Err(FlowError::TooFewOutbound { count: 1, min: 10 })
        );
    }

    /// **NFDU-03** — ratio exceeded rejected.
    #[test]
    fn nfdu_03_ratio_exceeded_rejected() {
        let mut obs_vec: Vec<FlowObservation> = (0..50)
            .map(|i| obs(FlowDirection::Inbound, i as u64 * 10))
            .collect();
        for i in 0..10 {
            obs_vec.push(obs(FlowDirection::Outbound, 500 + i as u64 * 10));
        }
        assert!(matches!(
            validate_flow_uniformity(&obs_vec),
            Err(FlowError::RatioExceeded { .. })
        ));
    }

    /// **NFDU-04** — too many observations rejected.
    #[test]
    fn nfdu_04_too_many_rejected() {
        let obs_vec: Vec<FlowObservation> = (0..=NFDU_MAX_OBSERVATIONS as u64)
            .map(|i| obs(FlowDirection::Inbound, i))
            .collect();
        assert_eq!(validate_flow_uniformity(&obs_vec), Err(FlowError::TooManyObservations));
    }

    /// **NFDU-05** — timestamps not increasing rejected.
    #[test]
    fn nfdu_05_timestamps_rejected() {
        let obs_vec = vec![
            obs(FlowDirection::Inbound, 200),
            obs(FlowDirection::Outbound, 100),
        ];
        assert_eq!(validate_flow_uniformity(&obs_vec), Err(FlowError::TimestampsNotIncreasing));
    }

    /// **NFDU-06** — balanced flow accepted.
    #[test]
    fn nfdu_06_balanced_accepted() {
        let flow: Vec<FlowObservation> = (0..20)
            .flat_map(|i| {
                let base = i as u64 * 100;
                vec![obs(FlowDirection::Inbound, base), obs(FlowDirection::Outbound, base + 1)]
            })
            .collect();
        assert_eq!(validate_flow_uniformity(&flow), Ok(()));
    }

    /// **NFDU-07** — valid flow accepted.
    #[test]
    fn nfdu_07_valid_accepted() {
        assert_eq!(validate_flow_uniformity(&balanced_flow()), Ok(()));
    }

    /// **NFDU-08** — empty accepted.
    #[test]
    fn nfdu_08_empty_accepted() {
        assert_eq!(validate_flow_uniformity(&[]), Ok(()));
    }

    /// **NFDU-09** — minimum boundary accepted.
    #[test]
    fn nfdu_09_min_boundary_accepted() {
        let flow: Vec<FlowObservation> = (0..10)
            .flat_map(|i| {
                let base = i as u64 * 100;
                vec![obs(FlowDirection::Inbound, base), obs(FlowDirection::Outbound, base + 1)]
            })
            .collect();
        assert_eq!(validate_flow_uniformity(&flow), Ok(()));
    }

    /// **NFDU-10** — 2:1 ratio accepted.
    #[test]
    fn nfdu_10_ratio_2_1_accepted() {
        let mut flow = Vec::new();
        for i in 0..20 {
            flow.push(obs(FlowDirection::Inbound, i as u64 * 10));
        }
        for i in 0..10 {
            flow.push(obs(FlowDirection::Outbound, 200 + i as u64 * 10));
        }
        assert_eq!(validate_flow_uniformity(&flow), Ok(()));
    }
}
