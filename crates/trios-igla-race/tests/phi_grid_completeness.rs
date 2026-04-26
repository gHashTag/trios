//! IGLA Race — V5: Phi-Grid Completeness Test (NEEDLE-RUSH L-V5)

use trios_igla_race::grid::{phi_grid_iter, PhiGridConfig, PHI};

#[derive(Debug, Clone, PartialEq)]
struct HistoricalConfig {
    d_model: usize,
    lr: f64,
    jepa_weight: f64,
    nca_weight: f64,
    bpb: f64,
    steps: usize,
    seed: u64,
}

fn phi_config_dominates(phi: &PhiGridConfig, hist: &HistoricalConfig) -> bool {
    let model_ok = phi.d_model >= hist.d_model || phi.d_model >= 384;
    let lr_ok = phi.lr() <= hist.lr * 1.2;
    let jepa_ok = phi.jepa_weight >= hist.jepa_weight * 0.8;
    model_ok && lr_ok && jepa_ok
}

#[test]
fn falsify_phi_grid_excludes_champion() {
    let champion = HistoricalConfig {
        d_model: 384,
        lr: 0.005,
        jepa_weight: 0.75,
        nca_weight: 0.5,
        bpb: 2.1697,
        steps: 60000,
        seed: 43,
    };

    let phi_configs: Vec<_> = phi_grid_iter().collect();
    let dominates = phi_configs
        .iter()
        .any(|phi| phi_config_dominates(phi, &champion));

    assert!(
        dominates,
        "phi-grid excludes champion config! Champion: d_model={}, lr={}",
        champion.d_model, champion.lr
    );
}

#[test]
fn falsify_phi_grid_d_model_range() {
    let phi_configs: Vec<_> = phi_grid_iter().collect();
    let phi_d_models: Vec<_> = phi_configs.iter().map(|c| c.d_model).collect();
    
    let phi_min = *phi_d_models.iter().min().unwrap();
    let phi_max = *phi_d_models.iter().max().unwrap();
    
    assert!(phi_min <= 64, "phi-grid min d_model too large: {}", phi_min);
    assert!(phi_max >= 384, "phi-grid max d_model too small: {}", phi_max);
}

#[test]
fn falsify_phi_grid_lr_covers_champion() {
    const CHAMPION_LR: f64 = 0.004;
    const TOLERANCE: f64 = 0.002;

    let phi_configs: Vec<_> = phi_grid_iter().collect();
    let lr_in_range = phi_configs
        .iter()
        .any(|c| (c.lr() - CHAMPION_LR).abs() < TOLERANCE);

    assert!(lr_in_range, "phi-grid LR range excludes champion LR");
}

#[test]
fn falsify_phi_resonance_violation() {
    let phi_configs: Vec<_> = phi_grid_iter().take(10).collect();
    
    for cfg in &phi_configs {
        let ratio = cfg.d_model as f64 / 64.0;
        let k = ratio.log(PHI).round() as i32;
        let expected = 64.0 * PHI.powi(k);
        let diff = (cfg.d_model as f64 - expected).abs() / expected;
        
        assert!(diff < 0.15, "d_model {} not phi-resonant", cfg.d_model);
    }
}

#[test]
fn falsify_grid_compression() {
    const NAIVE_GRID_SIZE: usize = 3_usize.pow(7);
    let phi_size = trios_igla_race::grid::phi_grid_size();
    
    let compression = NAIVE_GRID_SIZE as f64 / phi_size as f64;
    assert!(compression >= 10.0, "compression {}× below 10×", compression);
    assert!(phi_size >= 64, "phi-grid too small: {}", phi_size);
}

#[test]
fn falsify_golden_config_invalid() {
    let golden = phi_grid_iter().next().unwrap();
    
    assert!(golden.is_golden(), "first config must be golden");
    assert_eq!(golden.phi_band(), 0, "golden must be in band 0");
    assert_eq!(golden.d_model, 64, "golden d_model should be 64");
    assert!(golden.lr() <= 0.005, "golden LR should be conservative");
    assert!(golden.jepa_weight > 0.0, "jepa_weight must be positive");
    assert!(golden.nca_weight > 0.0, "nca_weight must be positive");
}

#[test]
fn falsify_phi_grid_nondeterministic() {
    let a: Vec<_> = phi_grid_iter().take(100).collect();
    let b: Vec<_> = phi_grid_iter().take(100).collect();
    assert_eq!(a, b, "phi-grid must be deterministic");
}

#[test]
fn falsify_phi_band_distribution() {
    let configs: Vec<_> = phi_grid_iter().take(25).collect();
    let mut band_counts = [0; 5];
    
    for cfg in &configs {
        band_counts[cfg.phi_band()] += 1;
    }
    
    for (i, &count) in band_counts.iter().enumerate() {
        assert!(count >= 4, "phi-band {} under-represented: {}", i, count);
    }
}
