//! Victory detection — BPB victory condition checking.
//!
//! Defines the victory condition for IGLA RACE and provides
//! statistical verification (Welch t-test).

use std::collections::HashMap;

use super::invariants::{PHI, PHI_INV, INV2_BPB_PRUNE_THRESHOLD};

/// Victory BPB target.
///
/// A trial achieves victory when BPB < 1.5.
/// L-R14 anchor: This matches `hive_automaton::BPB_VICTORY_TARGET`.
pub const IGLA_TARGET_BPB: f64 = 1.5;

/// Victory seed target.
pub const VICTORY_SEED_TARGET: u64 = 100;

/// JEPA proxy BPB floor (from L14 specification).
pub const JEPA_PROXY_BPB_FLOOR: f64 = 2.0;

/// Statistical strength threshold.
///
/// A result is considered strong if it passes Welch t-test
/// with 95% confidence against baseline.
pub const STAT_STRENGTH_THRESHOLD: f64 = 0.95;

/// Victory report with statistical verification.
#[derive(Debug, Clone)]
pub struct VictoryReport {
    /// Trial ID that achieved victory
    pub trial_id: u64,
    /// Final BPB achieved
    pub final_bpb: f64,
    /// Whether victory is statistically significant
    pub stat_significant: bool,
    /// Number of trials run
    pub trials_count: u64,
    /// Welch t-test report
    pub ttest: Option<TtestReport>,
}

/// Welch t-test report.
#[derive(Debug, Clone)]
pub struct TtestReport {
    /// t-statistic
    pub t_stat: f64,
    /// p-value
    pub p_value: f64,
    /// Degrees of freedom
    pub df: f64,
    /// Whether null hypothesis is rejected
    pub rejected: bool,
    /// Confidence level achieved
    pub confidence: f64,
}

/// Victory error.
#[derive(Debug, thiserror::Error)]
pub enum VictoryError {
    #[error("Insufficient samples for t-test: {0}")]
    InsufficientSamples(usize),

    #[error("Invalid baseline BPB: {0}")]
    InvalidBaseline(f64),
}

/// Check if a BPB value achieves victory.
///
/// Returns true if bpb < IGLA_TARGET_BPB.
pub fn is_victory(bpb: f64) -> bool {
    bpb < IGLA_TARGET_BPB
}

/// Check victory with full verification.
///
/// This function:
/// 1. Checks if BPB is below victory threshold
/// 2. Performs Welch t-test against baseline if available
/// 3. Returns victory report with statistical significance
pub fn check_victory(
    trial_id: u64,
    bpb: f64,
    baseline_bpbs: &[f64],
) -> Result<VictoryReport, VictoryError> {
    // Check victory condition
    if !is_victory(bpb) {
        return Ok(VictoryReport {
            trial_id,
            final_bpb: bpb,
            stat_significant: false,
            trials_count: baseline_bpbs.len() as u64 + 1,
            ttest: None,
        });
    }

    // Perform statistical verification if baseline available
    let ttest = if baseline_bpbs.len() >= 2 {
        Some(welch_ttest(bpb, baseline_bpbs)?)
    } else {
        None
    };

    let stat_significant = ttest.as_ref()
        .map(|tt| tt.confidence >= STAT_STRENGTH_THRESHOLD)
        .unwrap_or(false);

    Ok(VictoryReport {
        trial_id,
        final_bpb: bpb,
        stat_significant,
        trials_count: baseline_bpbs.len() as u64 + 1,
        ttest,
    })
}

/// Calculate statistical strength of a result.
///
/// Returns a confidence score 0-1 based on:
/// - How far below victory threshold
/// - Statistical significance
pub fn stat_strength(bpb: f64, ttest: Option<&TtestReport>) -> f64 {
    // Base strength from BPB improvement
    let improvement = (IGLA_TARGET_BPB - bpb) / IGLA_TARGET_BPB;
    let base_strength = improvement.clamp(0.0, 1.0);

    // Boost by statistical significance
    match ttest {
        Some(tt) => base_strength * tt.confidence,
        None => base_strength * 0.5,  // Mid confidence without t-test
    }
}

/// Perform Welch's t-test (unequal variances).
///
/// Tests if sample mean is significantly different from baseline.
pub fn welch_ttest(
    sample: f64,
    baseline: &[f64],
) -> Result<TtestReport, VictoryError> {
    if baseline.is_empty() {
        return Err(VictoryError::InsufficientSamples(0));
    }

    // Sample statistics
    let n1 = 1; // Single sample
    let mean1 = sample;
    let var1 = 0.0; // Single sample has no variance

    // Baseline statistics
    let n2 = baseline.len();
    let mean2: f64 = baseline.iter().sum::<f64>() / n2 as f64;
    let var2: f64 = baseline.iter()
        .map(|x| (x - mean2).powi(2))
        .sum::<f64>() / (n2 - 1) as f64;

    // Welch's t-test
    let se = (var1 / n1 as f64 + var2 / n2 as f64).sqrt();
    let t_stat = (mean1 - mean2) / se;

    // Degrees of freedom (Welch-Satterthwaite equation)
    let df = (var1 / n1 as f64 + var2 / n2 as f64).powi(2)
        / ((var1 / n1 as f64).powi(2) / (n1 - 1) as f64
            + (var2 / n2 as f64).powi(2) / (n2 - 1) as f64);

    // Two-tailed p-value (approximate)
    // Using t-distribution approximation
    let p_value = 2.0 * (1.0 - t_distribution_cdf(t_stat.abs(), df));

    Ok(TtestReport {
        t_stat,
        p_value,
        df,
        rejected: p_value < 0.05,
        confidence: if p_value < 0.05 { 0.95 }
                   else if p_value < 0.10 { 0.90 }
                   else if p_value < 0.20 { 0.80 }
                   else { 0.5 },
    })
}

/// Approximate t-distribution CDF.
///
/// Uses approximation for computational efficiency.
fn t_distribution_cdf(t: f64, df: f64) -> f64 {
    // Normal approximation for large df
    if df > 30.0 {
        let x = (t * (df / (df + t * t)).sqrt()).sqrt();
        return normal_cdf(x);
    }

    // For small df, use approximation
    // TODO: Implement proper t-distribution
    normal_cdf(t)
}

/// Standard normal CDF approximation.
fn normal_cdf(x: f64) -> f64 {
    const A1: f64 = 0.254829592;
    const A2: f64 = -0.284496736;
    const A3: f64 = 1.421413741;
    const A4: f64 = -1.453152027;
    const A5: f64 = 1.061405429;
    const P: f64 = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let k = 1.0 / (1.0 + P * x.abs());

    let w = 1.0 - (((((A5 * k + A4) * k) + A3) * k + A2) * k + A1) * k.powf(-5.0);
    0.5 * (1.0 + sign * w)
}

/// Integer power for f64.
fn powi(base: f64, exp: i32) -> f64 {
    let mut result = 1.0;
    for _ in 0..exp {
        result *= base;
    }
    result
}

/// Seed result for training.
#[derive(Debug, Clone)]
pub struct SeedResult {
    pub seed: u64,
    pub bpb: f64,
    pub victory: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_victory() {
        assert!(is_victory(1.4));
        assert!(!is_victory(1.5));
        assert!(!is_victory(2.0));
    }

    #[test]
    fn test_stat_strength() {
        let strong = stat_strength(1.0, Some(&TtestReport {
            t_stat: 3.0,
            p_value: 0.01,
            df: 10.0,
            rejected: true,
            confidence: 0.95,
        }));
        assert!(strong > 0.9);

        let weak = stat_strength(1.4, None);
        assert!(weak > 0.5);
        assert!(weak < 0.9);
    }

    #[test]
    fn test_welch_ttest_rejects_null() {
        let baseline = vec![3.0, 3.1, 2.9, 3.0, 3.2];
        let result = welch_ttest(1.5, &baseline).unwrap();
        assert!(result.rejected);
        assert!(result.confidence >= 0.90);
    }
}
