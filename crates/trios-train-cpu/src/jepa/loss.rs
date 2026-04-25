//! TASK-5A.4 — JEPA Loss Computation
//!
//! L2 normalisation + MSE prediction loss + variance anti-collapse term.
//! Theory: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/

#[derive(Debug, Clone, Copy)]
pub struct JepaLossConfig {
    pub use_l2_normalization: bool,
    pub variance_weight: f64,
}

impl Default for JepaLossConfig {
    fn default() -> Self {
        Self { use_l2_normalization: true, variance_weight: 0.01 }
    }
}

#[derive(Debug, Clone)]
pub struct JepaLoss {
    pub total: f64,
    pub prediction: f64,
    pub variance: f64,
}

pub fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    if norm > 1e-8 { for x in v.iter_mut() { *x /= norm; } }
}

pub fn l2_normalized(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    if norm > 1e-8 { v.iter().map(|x| x / norm).collect() } else { v.to_vec() }
}

pub fn compute_jepa_loss(predicted: &[f32], target: &[f32], config: JepaLossConfig) -> JepaLoss {
    assert_eq!(predicted.len(), target.len());
    let (pred, tgt) = if config.use_l2_normalization {
        (l2_normalized(predicted), l2_normalized(target))
    } else {
        (predicted.to_vec(), target.to_vec())
    };
    let prediction_loss = if pred.is_empty() { 0.0 } else {
        pred.iter().zip(tgt.iter())
            .map(|(p, t)| (*p - *t).powi(2) as f64)
            .sum::<f64>() / pred.len() as f64
    };
    let variance = if tgt.is_empty() { 0.0 } else {
        let mean = tgt.iter().sum::<f32>() as f64 / tgt.len() as f64;
        tgt.iter().map(|t| (*t as f64 - mean).powi(2)).sum::<f64>() / tgt.len() as f64
    };
    JepaLoss {
        total: prediction_loss - config.variance_weight * variance,
        prediction: prediction_loss,
        variance,
    }
}

/// MSE gradient: d_loss/d_pred = 2*(pred - target) / n
pub fn jepa_mse_grad(predicted: &[f32], target: &[f32]) -> Vec<f32> {
    let n = predicted.len().max(1) as f32;
    predicted.iter().zip(target.iter())
        .map(|(p, t)| 2.0 * (p - t) / n)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_normalize() {
        let mut v = vec![3.0_f32, 4.0_f32];
        l2_normalize(&mut v);
        let norm: f32 = v.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "norm={norm}");
        assert!((v[0] - 0.6).abs() < 1e-5);
        assert!((v[1] - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_l2_normalize_zero_vector() {
        let mut v = vec![0.0_f32; 4];
        l2_normalize(&mut v);
        assert!(v.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn test_jepa_loss_identical_vectors() {
        let v = vec![1.0_f32, 2.0_f32, 3.0_f32];
        let loss = compute_jepa_loss(&v, &v, JepaLossConfig::default());
        assert!(loss.prediction.abs() < 1e-6);
    }

    #[test]
    fn test_anti_collapse_variance() {
        let pred = vec![0.5_f32, 0.3_f32, 0.8_f32, 0.1_f32];
        let tgt = vec![0.2_f32, 0.9_f32, 0.4_f32, 0.7_f32];
        let loss = compute_jepa_loss(&pred, &tgt, JepaLossConfig::default());
        assert!(loss.variance > 0.0);
    }
}
