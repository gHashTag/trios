//! Schedule-Free + WSD Learning Rate Schedules (P3 Lab)
//!
//! Schedule-Free AdamW eliminates the need for explicit LR schedules
//! by maintaining an interpolation between training and momentum state.
//!
//! WSD (Warmup-Stable-Decay) decouples decay timing from total steps.
//!
//! References:
//! - Defazio et al. 2024: "Schedule-Free Learning"
//! - Wen et al. 2024: WSD scheduling

use std::f64::consts::PI;

/// Schedule type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleType {
    /// Standard cosine decay
    Cosine,

    /// Schedule-Free (no explicit schedule)
    ScheduleFree,

    /// Warmup-Stable-Decay
    Wsd,
}

/// Schedule-Free state
#[derive(Debug, Clone)]
pub struct ScheduleFreeState {
    /// Interpolation parameter c_t
    c_t: f64,

    /// Momentum buffer (z_t in Schedule-Free paper)
    z: Vec<f32>,

    /// Training state (x_t in Schedule-Free paper)
    x: Vec<f32>,

    /// Current step
    step: usize,
}

impl ScheduleFreeState {
    pub fn new(param_count: usize) -> Self {
        Self {
            c_t: 0.0,
            z: vec![0.0; param_count],
            x: vec![0.0; param_count],
            step: 0,
        }
    }

    /// Update interpolation coefficient
    /// c_{t+1} = 1/(t+1)
    pub fn update_c(&mut self) {
        self.step += 1;
        self.c_t = 1.0 / (self.step as f64);
    }

    /// Interpolate between z and x
    /// y_t = (1 - beta1) * z_t + beta1 * x_t
    pub fn interpolate(&self, x: &[f32], z: &[f32], beta1: f64) -> Vec<f32> {
        x.iter().zip(z.iter())
            .map(|(&xi, &zi)| (1.0 - beta1) * zi + beta1 * xi)
            .collect()
    }
}

/// WSD (Warmup-Stable-Decay) schedule
pub struct WsdSchedule {
    /// Warmup steps
    warmup: usize,

    /// Stable steps
    stable: usize,

    /// Decay steps
    decay: usize,

    /// Total steps
    total: usize,
}

impl WsdSchedule {
    pub fn new(warmup: usize, stable: usize, decay: usize) -> Self {
        let total = warmup + stable + decay;
        Self { warmup, stable, decay, total }
    }

    /// Get learning rate at step t
    pub fn lr(&self, t: usize, base_lr: f64, min_lr: f64) -> f64 {
        if t < self.warmup {
            // Linear warmup
            base_lr * (t as f64 / self.warmup as f64)
        } else if t < self.warmup + self.stable {
            // Stable: constant LR
            base_lr
        } else {
            // Cosine decay over decay period
            let decay_start = self.warmup + self.stable;
            let decay_progress = (t - decay_start) as f64 / self.decay as f64;
            let cosine = 0.5 * (1.0 + (PI * decay_progress).cos());
            min_lr + (base_lr - min_lr) * cosine
        }
    }
}

/// Cosine decay schedule (baseline)
pub struct CosineSchedule {
    pub warmup: usize,
    pub total: usize,
}

impl CosineSchedule {
    pub fn lr(&self, t: usize, base_lr: f64, min_lr: f64) -> f64 {
        if t < self.warmup {
            base_lr * (t as f64 / self.warmup as f64)
        } else {
            let progress = (t - self.warmup) as f64 / (self.total - self.warmup) as f64;
            let cosine = 0.5 * (1.0 + (PI * progress).cos());
            min_lr + (base_lr - min_lr) * cosine
        }
    }
}

/// Unified LR schedule interface
pub fn get_lr(
    schedule: ScheduleType,
    t: usize,
    base_lr: f64,
    min_lr: f64,
    warmup: usize,
    total: usize,
) -> f64 {
    match schedule {
        ScheduleType::Cosine => {
            let cosine = CosineSchedule { warmup, total };
            cosine.lr(t, base_lr, min_lr)
        }
        ScheduleType::Wsd => {
            let stable = total - warmup - (total / 5);  // 20% decay
            let decay = total / 5;
            let wsd = WsdSchedule::new(warmup, stable, decay);
            wsd.lr(t, base_lr, min_lr)
        }
        ScheduleType::ScheduleFree => {
            // Schedule-Free uses constant LR with momentum-based adaptation
            if t < warmup {
                base_lr * (t as f64 / warmup as f64)
            } else {
                base_lr
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_warmup() {
        let lr = get_lr(ScheduleType::Cosine, 0, 0.1, 0.0, 100, 1000);
        assert_eq!(lr, 0.0);
        let lr = get_lr(ScheduleType::Cosine, 50, 0.1, 0.0, 100, 1000);
        assert_eq!(lr, 0.05);
        let lr = get_lr(ScheduleType::Cosine, 100, 0.1, 0.0, 100, 1000);
        assert_eq!(lr, 0.1);
    }

    #[test]
    fn test_cosine_decay() {
        let lr_100 = get_lr(ScheduleType::Cosine, 100, 0.1, 1e-5, 100, 1000);
        let lr_500 = get_lr(ScheduleType::Cosine, 500, 0.1, 1e-5, 100, 1000);
        let lr_999 = get_lr(ScheduleType::Cosine, 999, 0.1, 1e-5, 100, 1000);
        assert!(lr_500 < lr_100, "LR should decay");
        assert!(lr_999 < lr_500, "LR should continue decaying");
    }

    #[test]
    fn test_schedule_free_constant_lr() {
        let lr = get_lr(ScheduleType::ScheduleFree, 500, 0.1, 0.0, 100, 1000);
        assert_eq!(lr, 0.1);
    }

    #[test]
    fn test_wsd_three_phases() {
        let lr_warmup = get_lr(ScheduleType::Wsd, 50, 0.1, 1e-5, 100, 1000);
        let lr_stable = get_lr(ScheduleType::Wsd, 300, 0.1, 1e-5, 100, 1000);
        let lr_decay = get_lr(ScheduleType::Wsd, 900, 0.1, 1e-5, 100, 1000);

        assert_eq!(lr_warmup, 0.05, "Warmup should be linear");
        assert_eq!(lr_stable, 0.1, "Stable should be constant");
        assert!(lr_decay < lr_stable, "Decay should be lower");
    }

    #[test]
    fn test_schedule_free_state() {
        let mut state = ScheduleFreeState::new(10);
        assert_eq!(state.step, 0);

        state.update_c();
        assert_eq!(state.step, 1);
        assert_eq!(state.c_t, 1.0);

        state.update_c();
        assert_eq!(state.step, 2);
        assert_eq!(state.c_t, 0.5);
    }

    #[test]
    fn test_schedule_free_interpolate() {
        let x = vec![1.0f32, 2.0, 3.0];
        let z = vec![0.0f32, 0.0, 0.0];
        let state = ScheduleFreeState::new(3);
        let beta1 = 0.9;

        let y = state.interpolate(&x, &z, beta1);
        assert_eq!(y[0], 0.9);  // (1-0.9)*0 + 0.9*1
        assert_eq!(y[1], 1.8);
        assert_eq!(y[2], 2.7);
    }
}
