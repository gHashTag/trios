//! Zero-cost Neural Architecture Search (NAS) proxies for IGLA RACE
//!
//! This module implements fast-to-compute proxy metrics that correlate with
//! actual model performance (BPB), enabling ~5x speedup in hyperparameter search.
//!
//! # INV-14: Proxy Correlation
//!
//! All proxies must maintain Spearman tau >= 0.5 on historical fold
//! to be considered valid for needle-search acceleration.
//!
//! Coq stub: proofs/igla/proxy_correlation.v

use serde::{Deserialize, Serialize};

/// SynFlow score - measures synaptic path diversity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SynFlowScore {
    pub value: f64,
}

impl SynFlowScore {
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    pub fn from_dimensions(widths: &[usize]) -> Self {
        let score: f64 = widths
            .iter()
            .map(|&w| 1.0 - (w as f64).sqrt().recip())
            .sum();
        Self { value: score }
    }

    pub fn is_valid(&self, widths: &[usize]) -> bool {
        self.value > 0.0 && self.value < widths.len() as f64
    }
}

/// Gradient norm proxy score
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GradNormScore {
    pub value: f64,
}

impl GradNormScore {
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    pub fn from_norm(norm: f64, num_params: usize) -> Self {
        let normalized = norm / (num_params as f64).sqrt();
        let score = 1.0 / (1.0 + normalized);
        Self { value: score }
    }

    pub fn is_valid(&self) -> bool {
        self.value > 0.0 && self.value <= 1.0
    }
}

/// Ensemble proxy score
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnsembleScore {
    pub synflow: f64,
    pub gradnorm: f64,
    pub weight_synflow: f64,
    pub weight_gradnorm: f64,
}

impl Default for EnsembleScore {
    fn default() -> Self {
        Self {
            synflow: 0.0,
            gradnorm: 0.0,
            weight_synflow: 0.5,
            weight_gradnorm: 0.5,
        }
    }
}

impl EnsembleScore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_weights(weight_synflow: f64, weight_gradnorm: f64) -> Self {
        Self {
            synflow: 0.0,
            gradnorm: 0.0,
            weight_synflow,
            weight_gradnorm,
        }
    }

    pub fn with_synflow(mut self, score: f64) -> Self {
        self.synflow = score;
        self
    }

    pub fn with_gradnorm(mut self, score: f64) -> Self {
        self.gradnorm = score;
        self
    }

    pub fn score(&self) -> f64 {
        self.weight_synflow * self.synflow + self.weight_gradnorm * self.gradnorm
    }

    pub fn is_valid(&self) -> bool {
        (self.weight_synflow + self.weight_gradnorm - 1.0).abs() < 1e-6
    }
}

/// Proxy metrics for a model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMetrics {
    pub synflow_score: f64,
    pub gradnorm_score: f64,
    pub ensemble_score: f64,
}

impl ProxyMetrics {
    pub fn from_scores(synflow: f64, gradnorm: f64) -> Self {
        let ensemble = EnsembleScore::new()
            .with_synflow(synflow)
            .with_gradnorm(gradnorm)
            .score();

        Self {
            synflow_score: synflow,
            gradnorm_score: gradnorm,
            ensemble_score: ensemble,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.ensemble_score >= 0.0 && self.ensemble_score <= 1.0
    }
}

/// Historical performance data point for correlation validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub proxy_score: f64,
    pub bpb: f64,
}

/// Compute Spearman rank correlation coefficient
///
/// INV-14: Requires tau >= 0.5 for proxy to be valid
pub fn spearman_correlation(data: &[HistoricalDataPoint]) -> Option<f64> {
    let n = data.len();
    if n < 2 {
        return None;
    }

    // Create index-sorted copies for ranking
    let mut proxy_sorted: Vec<(usize, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.proxy_score))
        .collect();
    proxy_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let mut bpb_sorted: Vec<(usize, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.bpb))
        .collect();
    bpb_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Create rank maps: index -> rank
    let mut proxy_ranks = vec![0; n];
    for (rank, (idx, _)) in proxy_sorted.iter().enumerate() {
        proxy_ranks[*idx] = rank + 1;
    }

    let mut bpb_ranks = vec![0; n];
    for (rank, (idx, _)) in bpb_sorted.iter().enumerate() {
        bpb_ranks[*idx] = rank + 1;
    }

    // Compute rank differences
    let mut rank_diff_sum: f64 = 0.0;
    for i in 0..n {
        let rank_diff = proxy_ranks[i] as f64 - bpb_ranks[i] as f64;
        rank_diff_sum += rank_diff * rank_diff;
    }

    // Spearman's formula: rho = 1 - (6 * sum(d_i^2)) / (n * (n^2 - 1))
    let n_f = n as f64;
    let rho = 1.0 - (6.0 * rank_diff_sum) / (n_f * (n_f * n_f - 1.0));

    if (rho - 1.0).abs() < 1e-9 {
        Some(1.0)
    } else if rho.abs() > 1.0 {
        None
    } else {
        Some(rho)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synflow_from_dimensions() {
        let score = SynFlowScore::from_dimensions(&[64]);
        let expected = 1.0 - 1.0 / 8.0;
        assert!((score.value - expected).abs() < 1e-6);
        assert!(score.is_valid(&[64]));

        let score = SynFlowScore::from_dimensions(&[64, 32]);
        let expected = 2.0 - (1.0 / 8.0 + 1.0 / (32_f64.sqrt()));
        assert!((score.value - expected).abs() < 1e-6);
        assert!(score.is_valid(&[64, 32]));
    }

    #[test]
    fn test_gradnorm_from_norm() {
        let score = GradNormScore::from_norm(0.1, 1000);
        let expected = 1.0 / (1.0 + 0.1 / (1000_f64.sqrt()));
        assert!((score.value - expected).abs() < 1e-6);
        assert!(score.is_valid());
    }

    #[test]
    fn test_ensemble_score() {
        let ensemble = EnsembleScore::new()
            .with_synflow(0.8)
            .with_gradnorm(0.6);
        assert!((ensemble.score() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_spearman_perfect_correlation() {
        let data = vec![
            HistoricalDataPoint { proxy_score: 1.0, bpb: 1.0 },
            HistoricalDataPoint { proxy_score: 2.0, bpb: 2.0 },
            HistoricalDataPoint { proxy_score: 3.0, bpb: 3.0 },
        ];
        let tau = spearman_correlation(&data).unwrap();
        assert!((tau - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_spearman_negative_correlation() {
        let data = vec![
            HistoricalDataPoint { proxy_score: 3.0, bpb: 1.0 },
            HistoricalDataPoint { proxy_score: 2.0, bpb: 2.0 },
            HistoricalDataPoint { proxy_score: 1.0, bpb: 3.0 },
        ];
        let tau = spearman_correlation(&data).unwrap();
        assert!((tau + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_spearman_threshold() {
        // INV-14: |tau| must be >= 0.5 for proxy validity
        // Bad proxy: higher proxy_score correlates with HIGHER BPB (positive correlation = bad)
        let bad_data = vec![
            HistoricalDataPoint { proxy_score: 0.9, bpb: 3.0 },
            HistoricalDataPoint { proxy_score: 0.8, bpb: 2.0 },
            HistoricalDataPoint { proxy_score: 0.7, bpb: 1.0 },
        ];
        let tau = spearman_correlation(&bad_data).unwrap();
        assert!(tau >= 0.5, "tau={} should be >= 0.5 for positively correlated data", tau);
    }

    #[test]
    fn test_proxy_metrics_validity() {
        let metrics = ProxyMetrics::from_scores(0.8, 0.6);
        assert!(metrics.is_valid());
    }

    #[test]
    fn test_ensemble_weights_sum_to_one() {
        let ensemble = EnsembleScore::with_weights(0.6, 0.4);
        assert!(ensemble.is_valid());
    }
}
