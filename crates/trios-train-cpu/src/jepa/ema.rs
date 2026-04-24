//! TASK-5A.2 — EMA Target Encoder
//!
//! theta_target = tau * theta_target + (1 - tau) * theta_online
//! Params: ema_start=0.996, ema_end=1.0
//! Theory: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/

#[derive(Debug, Clone, Copy)]
pub struct EmaConfig {
    pub start: f64,
    pub end: f64,
    pub ramp_steps: usize,
}

impl Default for EmaConfig {
    fn default() -> Self {
        Self { start: 0.996, end: 1.0, ramp_steps: 30_000 }
    }
}

pub struct EmaTarget {
    config: EmaConfig,
    step: usize,
}

impl EmaTarget {
    pub fn new(config: EmaConfig) -> Self { Self { config, step: 0 } }

    pub fn decay(&self) -> f64 {
        if self.step >= self.config.ramp_steps {
            self.config.end
        } else {
            let p = self.step as f64 / self.config.ramp_steps as f64;
            self.config.start + (self.config.end - self.config.start) * p
        }
    }

    pub fn update(&mut self, target: &mut [f32], online: &[f32]) {
        let tau = self.decay();
        ema_update(target, online, tau);
        self.step += 1;
    }

    pub fn reset(&mut self) { self.step = 0; }
    pub fn step(&self) -> usize { self.step }
}

pub fn ema_update(target: &mut [f32], online: &[f32], tau: f64) {
    assert_eq!(target.len(), online.len());
    let tau32 = tau as f32;
    let inv = (1.0 - tau) as f32;
    for (t, o) in target.iter_mut().zip(online.iter()) {
        *t = tau32 * (*t) + inv * (*o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_decay_schedule() {
        let cfg = EmaConfig { start: 0.5, end: 1.0, ramp_steps: 100 };
        let mut ema = EmaTarget::new(cfg);
        assert!((ema.decay() - 0.5).abs() < 1e-9);
        ema.step = 50;
        assert!((ema.decay() - 0.75).abs() < 1e-9);
        ema.step = 100;
        assert!((ema.decay() - 1.0).abs() < 1e-9);
        ema.step = 200;
        assert_eq!(ema.decay(), 1.0);
    }

    #[test]
    fn test_ema_update() {
        let mut t = vec![1.0_f32, 1.0_f32];
        ema_update(&mut t, &[2.0_f32, 0.0_f32], 0.9);
        assert!((t[0] - 1.1).abs() < 1e-5);
        assert!((t[1] - 0.9).abs() < 1e-5);
    }

    #[test]
    fn test_ema_reset() {
        let mut ema = EmaTarget::new(EmaConfig::default());
        ema.step = 1000;
        ema.reset();
        assert_eq!(ema.step(), 0);
    }

    #[test]
    fn test_ema_step_counter() {
        let mut ema = EmaTarget::new(EmaConfig::default());
        let mut t = vec![1.0_f32];
        ema.update(&mut t, &[1.0_f32]);
        ema.update(&mut t, &[1.0_f32]);
        assert_eq!(ema.step(), 2);
    }
}
