//! TASK-5A.6 — Multi-Objective Loss + ASHA Rung Schedules
//!
//! L_total = 0.5*NTP + 0.25*JEPA + 0.25*NCA
//! NCA entropy band: [1.5, 2.8] (trinity NCA Wave 8.5)
//! JEPA ASHA Law L-R10: minimum 3000-step first rung

#[derive(Debug, Clone, Copy)]
pub struct ObjectiveConfig {
    pub ntp_weight: f64,
    pub jepa_weight: f64,
    pub nca_weight: f64,
}

impl Default for ObjectiveConfig {
    fn default() -> Self {
        Self { ntp_weight: 0.5, jepa_weight: 0.25, nca_weight: 0.25 }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentLosses {
    pub ntp: f64,
    pub jepa: f64,
    pub nca: f64,
}

#[derive(Debug, Clone)]
pub struct CombinedLoss {
    pub total: f64,
    pub components: ComponentLosses,
}

pub fn compute_combined_loss(components: ComponentLosses, config: ObjectiveConfig) -> CombinedLoss {
    let total = components.ntp * config.ntp_weight
        + components.jepa * config.jepa_weight
        + components.nca * config.nca_weight;
    CombinedLoss { total, components }
}

/// NCA entropy constraint penalty (hard band [1.5, 2.8])
pub fn nca_entropy_constraint(entropy: f64) -> f64 {
    const MIN: f64 = 1.5;
    const MAX: f64 = 2.8;
    const SCALE: f64 = 100.0;
    if entropy < MIN { (MIN - entropy).powi(2) * SCALE }
    else if entropy > MAX { (entropy - MAX).powi(2) * SCALE }
    else { 0.0 }
}

/// ASHA rung schedule per architecture
/// Law L-R10: JEPA first rung = 3000 (1.4x slower convergence)
pub fn get_rung_schedule(arch: &str) -> Vec<u32> {
    match arch {
        "jepa"   => vec![3000, 9000, 27000],
        "attn"   => vec![1000, 3000, 9000, 27000],
        "hybrid" => vec![2000, 6000, 18000],
        _        => vec![1000, 3000, 9000, 27000],
    }
}

/// True if ASHA should skip this rung for arch
pub fn should_skip_rung(arch: &str, rung: u32) -> bool {
    arch == "jepa" && rung < 3000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combined_loss_weights() {
        let c = ComponentLosses { ntp: 2.0, jepa: 1.0, nca: 0.5 };
        let r = compute_combined_loss(c, ObjectiveConfig::default());
        assert!((r.total - 1.375).abs() < 1e-9, "total={}", r.total);
    }

    #[test]
    fn test_nca_entropy_in_band() {
        assert_eq!(nca_entropy_constraint(2.0), 0.0);
        assert_eq!(nca_entropy_constraint(1.5), 0.0);
        assert_eq!(nca_entropy_constraint(2.8), 0.0);
    }

    #[test]
    fn test_nca_entropy_below_band() {
        assert!((nca_entropy_constraint(1.0) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn test_nca_entropy_above_band() {
        assert!((nca_entropy_constraint(3.0) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_rung_schedule_jepa() {
        let r = get_rung_schedule("jepa");
        assert_eq!(r[0], 3000);
        assert!(r.iter().all(|&x| x >= 3000));
    }

    #[test]
    fn test_should_skip_rung() {
        assert!(should_skip_rung("jepa", 1000));
        assert!(!should_skip_rung("jepa", 3000));
        assert!(!should_skip_rung("ngram", 1000));
    }
}
