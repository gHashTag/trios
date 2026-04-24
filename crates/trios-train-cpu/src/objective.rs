//! Multi-objective loss computation
//!
//! Combines NTP (Next Token Prediction), JEPA, and NCA losses.

/// Multi-objective configuration
#[derive(Debug, Clone, Copy)]
pub struct ObjectiveConfig {
    pub ntp_weight: f64,
    pub jepa_weight: f64,
    pub nca_weight: f64,
}

impl Default for ObjectiveConfig {
    fn default() -> Self {
        Self {
            ntp_weight: 0.5,
            jepa_weight: 0.25,
            nca_weight: 0.25,
        }
    }
}

/// Component losses
#[derive(Debug, Clone)]
pub struct ComponentLosses {
    pub ntp: f64,
    pub jepa: f64,
    pub nca: f64,
}

impl ComponentLosses {
    pub fn new(ntp: f64, jepa: f64, nca: f64) -> Self {
        Self { ntp, jepa, nca }
    }
}

/// Combined loss result
#[derive(Debug, Clone)]
pub struct CombinedLoss {
    pub total: f64,
    pub components: ComponentLosses,
}

impl CombinedLoss {
    pub fn new(total: f64, components: ComponentLosses) -> Self {
        Self { total, components }
    }
}

/// Compute weighted combined loss
///
/// L_total = w_ntp * L_ntp + w_jepa * L_jepa + w_nca * L_nca
pub fn compute_combined_loss(
    components: ComponentLosses,
    config: ObjectiveConfig,
) -> CombinedLoss {
    let total = components.ntp * config.ntp_weight
        + components.jepa * config.jepa_weight
        + components.nca * config.nca_weight;

    CombinedLoss { total, components }
}

/// NCA entropy constraint
///
/// Enforces entropy in band [1.5, 2.8] as hard constraint
/// Returns penalty if entropy is outside the band
pub fn nca_entropy_constraint(entropy: f64) -> f64 {
    const MIN_ENTROPY: f64 = 1.5;
    const MAX_ENTROPY: f64 = 2.8;

    if entropy < MIN_ENTROPY {
        (MIN_ENTROPY - entropy).powi(2) * 100.0
    } else if entropy > MAX_ENTROPY {
        (entropy - MAX_ENTROPY).powi(2) * 100.0
    } else {
        0.0
    }
}

/// Check if NCA entropy is in acceptable band
pub fn nca_entropy_valid(entropy: f64) -> bool {
    (1.5..=2.8).contains(&entropy)
}

/// NCA configuration
#[derive(Debug, Clone, Copy)]
pub struct NcaConfig {
    pub grid_size: usize,    // 9x9 = 81
    pub k_states: usize,     // 9 or 27 (3^2, 3^3)
    pub rollout_steps: usize,
    pub entropy_min: f64,
    pub entropy_max: f64,
}

impl Default for NcaConfig {
    fn default() -> Self {
        Self {
            grid_size: 9,
            k_states: 9,
            rollout_steps: 64,
            entropy_min: 1.5,
            entropy_max: 2.8,
        }
    }
}

/// Hybrid training configuration
#[derive(Debug, Clone)]
pub struct HybridConfig {
    pub arch: String,
    pub d_model: usize,
    pub context: usize,
    pub lr: f64,
    pub objective: ObjectiveConfig,
    pub nca: Option<NcaConfig>,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            arch: "hybrid".to_string(),
            d_model: 384,
            context: 6,
            lr: 0.004,
            objective: ObjectiveConfig::default(),
            nca: Some(NcaConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_config_default() {
        let config = ObjectiveConfig::default();

        assert_eq!(config.ntp_weight, 0.5);
        assert_eq!(config.jepa_weight, 0.25);
        assert_eq!(config.nca_weight, 0.25);
    }

    #[test]
    fn test_compute_combined_loss() {
        let components = ComponentLosses::new(2.0, 1.0, 0.5);
        let config = ObjectiveConfig::default();

        let result = compute_combined_loss(components, config);

        // 2.0 * 0.5 + 1.0 * 0.25 + 0.5 * 0.25 = 1.0 + 0.25 + 0.125 = 1.375
        assert!((result.total - 1.375).abs() < 1e-10);
        assert_eq!(result.components.ntp, 2.0);
        assert_eq!(result.components.jepa, 1.0);
        assert_eq!(result.components.nca, 0.5);
    }

    #[test]
    fn test_nca_entropy_constraint_in_band() {
        let penalty = nca_entropy_constraint(2.0);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_nca_entropy_constraint_too_low() {
        let penalty = nca_entropy_constraint(1.0);
        // (1.5 - 1.0)^2 * 100 = 0.25 * 100 = 25
        assert_eq!(penalty, 25.0);
    }

    #[test]
    fn test_nca_entropy_constraint_too_high() {
        let penalty = nca_entropy_constraint(3.5);
        // (3.5 - 2.8)^2 * 100 = 0.49 * 100 = 49
        // Use approx for floating point comparison
        assert!((penalty - 49.0).abs() < 1e-9);
    }

    #[test]
    fn test_nca_entropy_valid() {
        assert!(nca_entropy_valid(2.0));
        assert!(nca_entropy_valid(1.5));
        assert!(nca_entropy_valid(2.8));
        assert!(!nca_entropy_valid(1.0));
        assert!(!nca_entropy_valid(3.0));
    }

    #[test]
    fn test_nca_config_default() {
        let config = NcaConfig::default();

        assert_eq!(config.grid_size, 9);
        assert_eq!(config.k_states, 9);
        assert_eq!(config.rollout_steps, 64);
        assert_eq!(config.entropy_min, 1.5);
        assert_eq!(config.entropy_max, 2.8);
    }

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();

        assert_eq!(config.arch, "hybrid");
        assert_eq!(config.d_model, 384);
        assert_eq!(config.context, 6);
        assert!(config.nca.is_some());
    }

    #[test]
    fn test_component_losses_new() {
        let losses = ComponentLosses::new(1.0, 2.0, 3.0);

        assert_eq!(losses.ntp, 1.0);
        assert_eq!(losses.jepa, 2.0);
        assert_eq!(losses.nca, 3.0);
    }

    #[test]
    fn test_combined_loss_new() {
        let components = ComponentLosses::new(1.0, 1.0, 1.0);
        let result = CombinedLoss::new(1.5, components);

        assert_eq!(result.total, 1.5);
        assert_eq!(result.components.ntp, 1.0);
    }

    #[test]
    fn test_objective_weights_sum_to_one() {
        let config = ObjectiveConfig::default();
        let sum = config.ntp_weight + config.jepa_weight + config.nca_weight;

        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_custom_objective_config() {
        let config = ObjectiveConfig {
            ntp_weight: 0.7,
            jepa_weight: 0.2,
            nca_weight: 0.1,
        };

        let components = ComponentLosses::new(1.0, 1.0, 1.0);
        let result = compute_combined_loss(components, config);

        assert!((result.total - 1.0).abs() < 1e-10);
    }
}
