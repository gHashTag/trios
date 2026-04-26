//! INV-14: Proxy Correlation Test
//!
//! Tests that zero-cost NAS proxy metrics correlate with actual model performance.
//! Spearman tau >= 0.5 on historical fold is required for acceleration validity.
//!
//! Coq proof: proofs/igla/proxy_correlation.v

use trios_igla_race::proxies::{
    EnsembleScore, GradNormScore, HistoricalDataPoint, ProxyMetrics,
    SynFlowScore, spearman_correlation,
};

/// Synthetic historical data for validation
///
/// In production, this would be loaded from actual training results.
/// Higher proxy_score should correlate with higher BPB (worse performance),
/// so that we can minimize both together.
fn synthetic_historical_data() -> Vec<HistoricalDataPoint> {
    vec![
        HistoricalDataPoint { proxy_score: 0.5, bpb: 1.8 },  // Low proxy, low BPB (good)
        HistoricalDataPoint { proxy_score: 0.6, bpb: 2.0 },
        HistoricalDataPoint { proxy_score: 0.7, bpb: 2.2 },
        HistoricalDataPoint { proxy_score: 0.8, bpb: 2.4 },
        HistoricalDataPoint { proxy_score: 0.9, bpb: 2.6 },  // High proxy, high BPB (bad)
    ]
}

/// Anti-correlated data (should fail INV-14)
///
/// This data has proxy_score increasing while bpb decreasing,
/// which creates negative correlation (tau < 0).
fn anti_correlated_data() -> Vec<HistoricalDataPoint> {
    vec![
        HistoricalDataPoint { proxy_score: 0.5, bpb: 2.6 },  // Low proxy, high BPB
        HistoricalDataPoint { proxy_score: 0.6, bpb: 2.4 },
        HistoricalDataPoint { proxy_score: 0.7, bpb: 2.2 },
        HistoricalDataPoint { proxy_score: 0.8, bpb: 2.0 },
        HistoricalDataPoint { proxy_score: 0.9, bpb: 1.8 },  // High proxy, low BPB
    ]
}

/// INV-14: Correlation threshold test
///
/// Spearman tau must be >= 0.5 for proxy to be valid
#[test]
fn inv14_correlation_threshold_met() {
    let data = synthetic_historical_data();
    let tau = spearman_correlation(&data).expect("Correlation should be computed");
    assert!(
        tau >= 0.5,
        "INV-14 FAILED: tau={} is below 0.5 threshold",
        tau
    );
}

/// INV-14: Anti-correlated data should fail
#[test]
fn inv14_anti_correlation_fails() {
    let data = anti_correlated_data();
    let tau = spearman_correlation(&data).expect("Correlation should be computed");
    assert!(
        tau < 0.0,
        "INV-14: Anti-correlated data should have negative tau, got {}",
        tau
    );
    assert!(
        tau < -0.5,
        "INV-14: Anti-correlated data should fail threshold, tau={}",
        tau
    );
}

/// INV-14: Perfect correlation case
#[test]
fn inv14_perfect_correlation() {
    let data = vec![
        HistoricalDataPoint { proxy_score: 1.0, bpb: 1.0 },
        HistoricalDataPoint { proxy_score: 2.0, bpb: 2.0 },
        HistoricalDataPoint { proxy_score: 3.0, bpb: 3.0 },
    ];
    let tau = spearman_correlation(&data).expect("Correlation should be computed");
    assert!((tau - 1.0).abs() < 1e-6, "Perfect correlation should have tau=1.0");
}

/// INV-14: Empty data should return None
#[test]
fn inv14_empty_data() {
    let data: Vec<HistoricalDataPoint> = vec![];
    let tau = spearman_correlation(&data);
    assert!(tau.is_none(), "Empty data should return None");
}

/// INV-14: Single point should return None
#[test]
fn inv14_single_point() {
    let data = vec![HistoricalDataPoint { proxy_score: 0.8, bpb: 2.0 }];
    let tau = spearman_correlation(&data);
    assert!(tau.is_none(), "Single point should return None");
}

/// SynFlow proxy: architecture diversity test
#[test]
fn proxy_synflow_architecture_diversity() {
    // Deeper network should have higher synflow score
    let single_layer = SynFlowScore::from_dimensions(&[64]);
    let two_layers = SynFlowScore::from_dimensions(&[64, 32]);
    let three_layers = SynFlowScore::from_dimensions(&[64, 32, 16]);

    assert!(single_layer.value < two_layers.value);
    assert!(two_layers.value < three_layers.value);
}

/// GradNorm proxy: gradient stability test
#[test]
fn proxy_gradnorm_gradient_stability() {
    // Lower gradient norm should give higher score
    let stable = GradNormScore::from_norm(0.1, 1000);
    let unstable = GradNormScore::from_norm(1.0, 1000);

    assert!(stable.value > unstable.value);
    assert!(stable.is_valid());
    assert!(unstable.is_valid());
}

/// Ensemble proxy: weighted combination test
#[test]
fn proxy_ensemble_weighted_combination() {
    let ensemble = EnsembleScore::with_weights(0.6, 0.4)
        .with_synflow(0.8)
        .with_gradnorm(0.6);

    assert!(ensemble.is_valid());
    let score = ensemble.score();
    assert!((score - 0.72).abs() < 1e-6); // 0.6*0.8 + 0.4*0.6 = 0.72
}

/// ProxyMetrics: validity check test
#[test]
fn proxy_metrics_validity() {
    let metrics = ProxyMetrics::from_scores(0.8, 0.6);
    assert!(metrics.is_valid());
    assert_eq!(metrics.synflow_score, 0.8);
    assert_eq!(metrics.gradnorm_score, 0.6);
    assert_eq!(metrics.ensemble_score, 0.7); // (0.8 + 0.6) / 2
}

/// INV-14: Real-world scenario test
///
/// Tests proxy behavior on realistic hyperparameter sweep data
#[test]
fn inv14_realistic_hyperparameter_sweep() {
    let data = vec![
        HistoricalDataPoint { proxy_score: 0.55, bpb: 1.85 },  // Best config (low proxy, low BPB)
        HistoricalDataPoint { proxy_score: 0.65, bpb: 1.95 },  // Good config
        HistoricalDataPoint { proxy_score: 0.75, bpb: 2.10 },  // Medium config
        HistoricalDataPoint { proxy_score: 0.85, bpb: 2.25 },  // Poor config
        HistoricalDataPoint { proxy_score: 0.90, bpb: 2.40 },  // Worst config (high proxy, high BPB)
    ];

    let tau = spearman_correlation(&data).expect("Correlation should be computed");
    assert!(
        tau >= 0.5,
        "INV-14 FAILED on realistic data: tau={} < 0.5",
        tau
    );
}

/// INV-14: Held-out validation test
///
/// Simulates training on one set of configs and validating on another
#[test]
fn inv14_held_out_validation() {
    let training_data = vec![
        HistoricalDataPoint { proxy_score: 0.6, bpb: 2.0 },
        HistoricalDataPoint { proxy_score: 0.7, bpb: 2.2 },
        HistoricalDataPoint { proxy_score: 0.8, bpb: 2.4 },
    ];

    let held_out_data = vec![
        HistoricalDataPoint { proxy_score: 0.5, bpb: 1.9 },
        HistoricalDataPoint { proxy_score: 0.9, bpb: 2.5 },
    ];

    // Training data should pass
    let train_tau = spearman_correlation(&training_data).unwrap();
    assert!(train_tau >= 0.5);

    // Held-out data should also maintain correlation
    let held_out_tau = spearman_correlation(&held_out_data).unwrap();
    assert!(
        held_out_tau >= 0.5,
        "INV-14 FAILED on held-out data: tau={} < 0.5",
        held_out_tau
    );
}

/// INV-14: Correlation consistency across subsets
///
/// Tests that correlation is stable across different data subsets
#[test]
fn inv14_correlation_stability() {
    let full_data = vec![
        HistoricalDataPoint { proxy_score: 0.4, bpb: 1.8 },
        HistoricalDataPoint { proxy_score: 0.5, bpb: 2.0 },
        HistoricalDataPoint { proxy_score: 0.6, bpb: 2.2 },
        HistoricalDataPoint { proxy_score: 0.7, bpb: 2.4 },
        HistoricalDataPoint { proxy_score: 0.8, bpb: 2.6 },
        HistoricalDataPoint { proxy_score: 0.9, bpb: 2.8 },
    ];

    let full_tau = spearman_correlation(&full_data).unwrap();

    // First half
    let first_half = &full_data[0..3];
    let first_tau = spearman_correlation(first_half).unwrap();

    // Second half
    let second_half = &full_data[3..6];
    let second_tau = spearman_correlation(second_half).unwrap();

    // All correlations should be high and stable
    assert!(full_tau >= 0.5);
    assert!(first_tau >= 0.5);
    assert!(second_tau >= 0.5);

    // Stability: correlation values should be within 0.2 of each other
    assert!((first_tau - second_tau).abs() < 0.2);
}

/// INV-14: Proxy rank ordering test
///
/// Tests that proxy correctly ranks architectures by expected performance
#[test]
fn inv14_proxy_rank_ordering() {
    let configs = vec![
        (vec![64], "single-layer"),
        (vec![64, 32], "two-layers"),
        (vec![64, 32, 16], "three-layers"),
    ];

    let mut scores: Vec<_> = configs
        .iter()
        .map(|(widths, _name)| {
            (
                widths.clone(),
                SynFlowScore::from_dimensions(widths).value,
            )
        })
        .collect();

    // Sort by score descending (higher is better)
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Three-layer should have highest score
    assert_eq!(scores[0].0, vec![64, 32, 16]);
}
