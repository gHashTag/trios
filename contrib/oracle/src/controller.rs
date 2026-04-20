use crate::{Action, OracleDecision};

pub struct OracleController {
    max_parallel: usize,
    active: usize,
    bpb_threshold: f32,
}

impl OracleController {
    pub fn new(max_parallel: usize, bpb_threshold: f32) -> Self {
        Self {
            max_parallel,
            active: 0,
            bpb_threshold,
        }
    }

    pub fn decide(&mut self, current_bpb: f32, pending_seeds: &[u64]) -> Vec<OracleDecision> {
        let mut decisions = Vec::new();

        if current_bpb < self.bpb_threshold && self.active > 0 {
            for _ in 0..self.active {
                decisions.push(OracleDecision {
                    action: Action::Kill,
                    seed: 0,
                    reason: format!(
                        "BPB {:.4} < threshold {:.4}",
                        current_bpb, self.bpb_threshold
                    ),
                });
            }
            self.active = 0;
            return decisions;
        }

        let to_spawn = self
            .max_parallel
            .saturating_sub(self.active)
            .min(pending_seeds.len());
        for &seed in &pending_seeds[..to_spawn] {
            decisions.push(OracleDecision {
                action: Action::Spawn,
                seed,
                reason: format!(
                    "BPB {:.4} >= threshold, parallel {}/{}",
                    current_bpb,
                    self.active + 1,
                    self.max_parallel
                ),
            });
            self.active += 1;
        }

        if decisions.is_empty() {
            decisions.push(OracleDecision {
                action: Action::Wait,
                seed: 0,
                reason: format!(
                    "Active: {}/{}, pending: {}",
                    self.active,
                    self.max_parallel,
                    pending_seeds.len()
                ),
            });
        }

        decisions
    }
}
