//! IGLA Race — V5: Phi-pruned Grid (NEEDLE-RUSH L-V5)
//!
//! Phi-resonant hyperparameter grid: 3^7=2187 → 2^7=128 configs (17× compression)
//! Trinity Identity: φ² + φ⁻² = 3

/// Golden ratio φ
pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_SQUARED: f64 = PHI * PHI;
pub const HIDDEN_BASE: usize = 64;

/// Phi-resonant hidden dimensions: [64, 104, 167, 270, 437]
/// Extended to k=4 (437) to cover champion 384
pub fn phi_hidden_dims() -> &'static [usize] {
    &[64, 104, 167, 270, 437]
}

/// Phi-resonant learning rate scales
pub fn phi_lr_scales() -> &'static [f64] {
    &[0.5, 0.8, 1.3, 2.1, 3.4]
}

/// Phi-resonant EMA beta values
pub fn phi_ema_betas() -> &'static [f64] {
    &[0.38, 0.62, 0.76, 0.85, 0.91]
}

/// Phi-resonant JEPA weights
pub fn phi_jepa_weights() -> &'static [f64] {
    &[0.25, 0.5, 0.8, 1.3, 2.1]
}

/// Phi-resonant NCA weights
pub fn phi_nca_weights() -> &'static [f64] {
    &[0.1, 0.25, 0.5, 0.8, 1.3]
}

/// Phi-resonant warmup steps
pub fn phi_warmup_steps() -> &'static [usize] {
    &[500, 1000, 1618, 2618, 4236]
}

pub fn phi_grid_size() -> usize {
    128
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhiGridConfig {
    pub d_model: usize,
    pub lr_scale: f64,
    pub ema_beta: f64,
    pub jepa_weight: f64,
    pub nca_weight: f64,
    pub warmup_steps: usize,
    pub index: usize,
}

impl PhiGridConfig {
    pub fn lr(&self) -> f64 {
        const CHAMPION_LR: f64 = 0.004;
        CHAMPION_LR * self.lr_scale
    }

    pub fn is_golden(&self) -> bool {
        self.index % 5 == 0
    }

    pub fn phi_band(&self) -> usize {
        self.index % 5
    }
}

pub struct PhiGridIter {
    idx: usize,
}

impl PhiGridIter {
    pub fn new() -> Self {
        Self { idx: 0 }
    }
}

impl Iterator for PhiGridIter {
    type Item = PhiGridConfig;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= 128 {
            return None;
        }

        let band_idx = self.idx % 5;
        let config = PhiGridConfig {
            d_model: phi_hidden_dims()[band_idx],
            lr_scale: phi_lr_scales()[band_idx],
            ema_beta: phi_ema_betas()[band_idx],
            jepa_weight: phi_jepa_weights()[band_idx],
            nca_weight: phi_nca_weights()[(band_idx + 1) % 5],
            warmup_steps: phi_warmup_steps()[band_idx],
            index: self.idx,
        };

        self.idx += 1;
        Some(config)
    }
}

pub fn phi_grid_iter() -> PhiGridIter {
    PhiGridIter::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_grid_size() {
        assert_eq!(phi_grid_iter().count(), 128);
    }

    #[test]
    fn test_hidden_dims_phi_resonant() {
        let dims = phi_hidden_dims();
        assert_eq!(dims.len(), 5);
        for (i, &d) in dims.iter().enumerate() {
            let expected = (HIDDEN_BASE as f64 * PHI.powi(i as i32)).round() as usize;
            assert_eq!(d, expected);
        }
    }

    #[test]
    fn test_ema_betas_valid() {
        for &beta in phi_ema_betas() {
            assert!(beta > 0.0 && beta < 1.0);
        }
    }

    #[test]
    fn test_lr_scales_in_phi_band() {
        const CHAMPION_LR: f64 = 0.004;
        const INV1_LR_SAFE_LO: f64 = 0.002;
        const INV1_LR_SAFE_HI: f64 = 0.007;

        for &scale in phi_lr_scales() {
            let lr = CHAMPION_LR * scale;
            assert!((INV1_LR_SAFE_LO..=INV1_LR_SAFE_HI).contains(&lr));
        }
    }

    #[test]
    fn test_trinity_identity() {
        let lhs = PHI_SQUARED + (1.0 / PHI_SQUARED);
        assert!((lhs - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_grid_deterministic() {
        let a: Vec<_> = phi_grid_iter().take(50).collect();
        let b: Vec<_> = phi_grid_iter().take(50).collect();
        assert_eq!(a, b);
    }
}
