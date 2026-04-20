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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_when_bpb_above_threshold() {
        let mut ctrl = OracleController::new(3, 0.5);
        let decisions = ctrl.decide(1.0, &[42, 84, 126]);
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].action, Action::Spawn);
        assert_eq!(decisions[0].seed, 42);
    }

    #[test]
    fn test_kill_when_bpb_below_threshold() {
        let mut ctrl = OracleController::new(3, 0.5);
        ctrl.decide(1.0, &[42, 84, 126]);
        let decisions = ctrl.decide(0.3, &[168]);
        assert!(decisions.iter().all(|d| d.action == Action::Kill));
    }

    #[test]
    fn test_wait_when_full() {
        let mut ctrl = OracleController::new(2, 0.5);
        ctrl.decide(1.0, &[42, 84]);
        let decisions = ctrl.decide(1.0, &[126]);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].action, Action::Wait);
    }

    #[test]
    fn test_no_kill_when_active_zero() {
        let mut ctrl = OracleController::new(3, 0.5);
        let decisions = ctrl.decide(0.1, &[]);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].action, Action::Wait);
    }

    #[test]
    fn test_partial_spawn() {
        let mut ctrl = OracleController::new(3, 0.5);
        let decisions = ctrl.decide(1.0, &[42]);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].seed, 42);
    }

    #[test]
    fn test_spawn_kill_spawn_cycle() {
        let mut ctrl = OracleController::new(2, 0.5);

        let d1 = ctrl.decide(1.0, &[10, 20]);
        assert_eq!(d1.len(), 2);
        assert_eq!(d1[0].action, Action::Spawn);

        let d2 = ctrl.decide(0.2, &[30]);
        assert!(d2.iter().all(|d| d.action == Action::Kill));

        let d3 = ctrl.decide(0.8, &[30, 40]);
        assert_eq!(d3.len(), 2);
        assert_eq!(d3[0].action, Action::Spawn);
    }

    #[test]
    fn test_decision_serialization() {
        let d = OracleDecision {
            action: Action::Spawn,
            seed: 42,
            reason: "test".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("\"Spawn\""));
        let parsed: OracleDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.action, Action::Spawn);
        assert_eq!(parsed.seed, 42);
    }
}
