//! # trios-phi-schedule
//!
//! φ-LR scheduler — golden ratio-based learning rate schedule.
//!
//! ## Issue #54: LR Schedule Calibration
//!
//! Three schedules for calibration:
//! - (a) flat_3e4: Constant LR
//! - (b) cosine_3e4_to_0: Cosine decay
//! - (c) phi_decay_3e4_to_alpha_phi: Phi-based decay to α_φ floor
//!
//! ## Key Scientific Finding (Issue #53)
//!
//! α_φ = 0.118034 is NOT a valid initial LR (BPB explodes to 18.60).
//! Hypothesis: α_φ serves as ASYMPTOTIC FLOOR in decay schedule.

use trios_physics::gf_constants;

/// LR schedule type for Issue #54 calibration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LrScheduleType {
    /// Constant LR 3e-4
    Flat,
    /// Cosine decay 3e-4 → 0
    Cosine,
    /// Phi decay 3e-4 → α_φ (hypothesis: α_φ as asymptotic floor)
    PhiDecay,
}

/// Compute φ-optimized learning rate schedule.
///
/// The learning rate decays according to the golden ratio φ:
/// ```text
/// LR = base_lr * φ^(-epoch / warmup)
/// ```
///
/// # Arguments
///
/// * `epoch` - Current training epoch (0-indexed)
/// * `base_lr` - Base learning rate
/// * `warmup` - Warmup period length (determines decay rate)
///
/// # Returns
///
/// The scheduled learning rate for the given epoch.
///
/// # Example
///
/// ```
/// use trios_phi_schedule::phi_schedule;
///
/// let lr = phi_schedule(10, 0.001, 20);
/// assert!(lr < 0.001); // LR should decay
/// ```
pub fn phi_schedule(epoch: usize, base_lr: f32, warmup: usize) -> f32 {
    let phi = gf_constants().phi as f32;
    let decay = phi.powf(-(epoch as f32 / warmup as f32));
    base_lr * decay
}

/// Issue #54: Flat LR schedule (baseline)
///
/// Constant learning rate 3e-4.
pub fn flat_lr(_step: usize, base_lr: f32) -> f32 {
    base_lr
}

/// Issue #54: Cosine LR schedule
///
/// Decays from base_lr to 0 using cosine curve.
pub fn cosine_lr(step: usize, max_steps: usize, base_lr: f32) -> f32 {
    let progress = step as f32 / max_steps as f32;
    let cosine = (std::f32::consts::PI * progress).cos();
    base_lr * (1.0 + cosine) / 2.0
}

/// Issue #54: Phi-decay LR schedule (hypothesis)
///
/// Decays from base_lr using φ-based decay.
/// Formula: LR = base_lr * φ^(-t/τ)
/// where t = (step - warmup) and τ = max_steps / (φ × 27)
///
/// This hypothesis: α_φ (≈0.118) relates to decay structure, not absolute LR.
/// The decay is bounded to prevent vanishing LR.
pub fn phi_decay_lr(step: usize, max_steps: usize, base_lr: f32, warmup_steps: usize) -> f32 {
    let phi = gf_constants().phi as f32;

    if step < warmup_steps {
        base_lr
    } else {
        let tau = max_steps as f32 / (phi * 27.0);
        let t = (step - warmup_steps) as f32 / tau;
        // Decay using phi^(-t/τ) which decreases since phi > 1
        let decay = phi.powf(-t.min(10.0));
        // Ensure LR doesn't vanish completely (1e-6 minimum)
        (base_lr * decay).max(1e-6)
    }
}

/// Unified LR scheduler for Issue #54 calibration
///
/// Select schedule type and compute LR for current step.
pub fn lr_schedule_54(schedule_type: LrScheduleType, step: usize, max_steps: usize) -> f32 {
    const BASE_LR: f32 = 3e-4;
    const WARMUP_STEPS: usize = 100;

    match schedule_type {
        LrScheduleType::Flat => flat_lr(step, BASE_LR),
        LrScheduleType::Cosine => cosine_lr(step, max_steps, BASE_LR),
        LrScheduleType::PhiDecay => phi_decay_lr(step, max_steps, BASE_LR, WARMUP_STEPS),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_schedule_no_decay_epoch_0() {
        let base_lr = 0.001f32;
        let lr = phi_schedule(0, base_lr, 10);
        assert_eq!(lr, base_lr, "Epoch 0 should return base_lr");
    }

    #[test]
    fn test_phi_schedule_decays_over_epochs() {
        let base_lr = 0.001f32;
        let warmup = 10;
        let lr_0 = phi_schedule(0, base_lr, warmup);
        let lr_10 = phi_schedule(10, base_lr, warmup);
        let lr_20 = phi_schedule(20, base_lr, warmup);

        assert!(lr_10 < lr_0, "LR should decay from epoch 0 to 10");
        assert!(lr_20 < lr_10, "LR should decay from epoch 10 to 20");
    }

    #[test]
    fn test_phi_schedule_phi_factor() {
        let base_lr = 1.0f32;
        let warmup = 1;
        let lr_0 = phi_schedule(0, base_lr, warmup);
        let lr_1 = phi_schedule(1, base_lr, warmup);
        let phi = gf_constants().phi as f32;

        assert!((lr_1 - lr_0 / phi).abs() < 1e-6, "LR should decay by factor of φ");
    }

    #[test]
    fn test_phi_schedule_zero_warmup() {
        let base_lr = 0.001f32;
        let lr = phi_schedule(5, base_lr, 0);
        // Division by zero should not panic; result may be NaN or inf
        assert!(lr.is_nan() || lr.is_infinite() || lr == 0.0);
    }

    // === Issue #54 tests ===

    #[test]
    fn test_flat_lr_constant() {
        let base_lr = 3e-4_f32;
        let lr_0 = flat_lr(0, base_lr);
        let lr_100 = flat_lr(100, base_lr);
        let lr_1000 = flat_lr(1000, base_lr);

        assert_eq!(lr_0, base_lr);
        assert_eq!(lr_100, base_lr);
        assert_eq!(lr_1000, base_lr);
    }

    #[test]
    fn test_cosine_lr_decay() {
        let base_lr = 3e-4_f32;
        let max_steps = 1000;

        let lr_0 = cosine_lr(0, max_steps, base_lr);
        let lr_500 = cosine_lr(500, max_steps, base_lr);
        let lr_999 = cosine_lr(999, max_steps, base_lr);

        assert_eq!(lr_0, base_lr, "Step 0 should return base_lr");
        assert!(lr_500 < lr_0, "LR should decay by midpoint");
        assert!(lr_999 < lr_500, "LR should continue decaying");
        assert!(lr_999 < 1e-6_f32, "Final LR should approach 0");
    }

    #[test]
    fn test_cosine_lr_monotonic_decay() {
        let base_lr = 1.0_f32;
        let max_steps = 1000;

        // Cosine decay should monotonically decrease
        let lr_0 = cosine_lr(0, max_steps, base_lr);
        let lr_25 = cosine_lr(250, max_steps, base_lr);
        let lr_50 = cosine_lr(500, max_steps, base_lr);
        let lr_75 = cosine_lr(750, max_steps, base_lr);
        let lr_100 = cosine_lr(1000, max_steps, base_lr);

        assert_eq!(lr_0, base_lr, "Step 0 should return base_lr");
        assert!(lr_25 < lr_0, "LR should decay from start");
        assert!(lr_50 < lr_25, "LR should continue decaying");
        assert!(lr_75 < lr_50, "LR should continue decaying");
        assert!(lr_100 <= lr_75, "LR should reach minimum at end");
        assert!(lr_100 < 1e-6_f32, "Final LR should approach 0");
    }

    #[test]
    fn test_phi_decay_lr_warmup() {
        let base_lr = 3e-4_f32;
        let max_steps = 1000;
        let warmup = 100;

        // During warmup, LR should be constant
        let lr_0 = phi_decay_lr(0, max_steps, base_lr, warmup);
        let lr_50 = phi_decay_lr(50, max_steps, base_lr, warmup);
        let lr_99 = phi_decay_lr(99, max_steps, base_lr, warmup);

        assert_eq!(lr_0, base_lr);
        assert_eq!(lr_50, base_lr);
        assert_eq!(lr_99, base_lr);
    }

    #[test]
    fn test_phi_decay_lr_post_warmup() {
        let base_lr = 3e-4_f32;
        let max_steps = 1000;
        let warmup = 100;

        let lr_warmup = phi_decay_lr(99, max_steps, base_lr, warmup);
        // Use step 200 to ensure decay is visible (tau ≈ 23)
        let lr_decayed = phi_decay_lr(200, max_steps, base_lr, warmup);
        let lr_later = phi_decay_lr(500, max_steps, base_lr, warmup);

        assert_eq!(lr_warmup, base_lr, "Last warmup step should be base_lr");
        // LR should decay after warmup (decrease from base_lr)
        assert!(lr_decayed < base_lr, "LR should decay after warmup");
        // LR should continue decaying or stay at floor
        assert!(lr_later <= lr_decayed, "LR should continue decaying");
    }

    #[test]
    fn test_lr_schedule_54_flat() {
        let max_steps = 1000;
        let lr_0 = lr_schedule_54(LrScheduleType::Flat, 0, max_steps);
        let lr_500 = lr_schedule_54(LrScheduleType::Flat, 500, max_steps);
        let lr_999 = lr_schedule_54(LrScheduleType::Flat, 999, max_steps);

        assert_eq!(lr_0, 3e-4_f32);
        assert_eq!(lr_500, 3e-4_f32);
        assert_eq!(lr_999, 3e-4_f32);
    }

    #[test]
    fn test_lr_schedule_54_cosine() {
        let max_steps = 1000;
        let lr_0 = lr_schedule_54(LrScheduleType::Cosine, 0, max_steps);
        let lr_999 = lr_schedule_54(LrScheduleType::Cosine, 999, max_steps);

        assert_eq!(lr_0, 3e-4_f32);
        assert!(lr_999 < 3e-4_f32);
    }

    #[test]
    fn test_lr_schedule_54_phi_decay() {
        let max_steps = 1000;
        let lr_99 = lr_schedule_54(LrScheduleType::PhiDecay, 99, max_steps);
        let lr_100 = lr_schedule_54(LrScheduleType::PhiDecay, 100, max_steps);
        let lr_500 = lr_schedule_54(LrScheduleType::PhiDecay, 500, max_steps);

        assert_eq!(lr_99, 3e-4_f32, "Step 99 should be in warmup");
        // LR changes after warmup
        assert!(lr_100 != 3e-4_f32 || lr_500 != 3e-4_f32);
    }
}
