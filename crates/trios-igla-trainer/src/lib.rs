pub mod audit;
pub mod config;
pub mod schedule;
pub mod jepa_runner;

pub use audit::AuditLog;
pub use config::TrainConfig;
pub use schedule::{Schedule, StepResult};
pub use jepa_runner::{run_jepa_training, JepaTrainArgs};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScheduleType;

    #[test]
    fn train_config_default_values() {
        let cfg = TrainConfig::default();
        assert_eq!(cfg.model_id, "igla-gf16");
        assert_eq!(cfg.steps, 1000);
        assert_eq!(cfg.batch_size, 4);
        assert_eq!(cfg.seq_len, 128);
        assert_eq!(cfg.schedule, ScheduleType::Flat3e4);
        assert_eq!(cfg.seed, 42);
        assert_eq!(cfg.repo, "gHashTag/trios");
        assert_eq!(cfg.branch, "main");
    }

    #[test]
    fn schedule_type_serde_roundtrip() {
        let variants = [
            ScheduleType::Flat3e4,
            ScheduleType::Cosine,
            ScheduleType::PhiWarmup,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: ScheduleType = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, back);
        }
    }

    #[test]
    fn train_config_serde_roundtrip() {
        let cfg = TrainConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: TrainConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.model_id, back.model_id);
        assert_eq!(cfg.steps, back.steps);
        assert_eq!(cfg.batch_size, back.batch_size);
        assert_eq!(cfg.seed, back.seed);
    }

    #[test]
    fn schedule_flat_lr_is_constant() {
        let s = Schedule::Flat3e4;
        assert_eq!(s.lr(0, 1000), 3e-4);
        assert_eq!(s.lr(500, 1000), 3e-4);
        assert_eq!(s.lr(999, 1000), 3e-4);
    }

    #[test]
    fn schedule_cosine_lr_at_start() {
        let s = Schedule::Cosine;
        let lr_start = s.lr(0, 1000);
        assert!((lr_start - 3e-4).abs() < 1e-8);
    }

    #[test]
    fn schedule_cosine_lr_decays() {
        let s = Schedule::Cosine;
        let lr_start = s.lr(0, 1000);
        let lr_mid = s.lr(500, 1000);
        assert!(lr_mid < lr_start);
    }

    #[test]
    fn schedule_phi_warmup_increases() {
        let s = Schedule::PhiWarmup;
        let lr_early = s.lr(10, 1000);
        let lr_late = s.lr(900, 1000);
        assert!(lr_late > lr_early);
    }

    #[test]
    fn schedule_bpb_from_loss() {
        let s = Schedule::Flat3e4;
        let bpb = s.bpb_from_loss(1.0);
        assert!((bpb - (1.0f32 / std::f32::consts::LN_2)).abs() < 1e-6);
    }

    #[test]
    fn audit_log_new() {
        let log = AuditLog::new("test-model", 42, 100, "abc123");
        assert_eq!(log.model_id, "test-model");
        assert_eq!(log.seed, 42);
        assert_eq!(log.steps, 100);
        assert_eq!(log.git_sha, "abc123");
        assert!(log.results.is_empty());
        assert!(log.timestamp > 0);
    }

    #[test]
    fn audit_log_record_accumulates() {
        let mut log = AuditLog::new("m", 0, 10, "sha");
        log.record(1, 2.5, 3.6, 3e-4);
        log.record(2, 1.8, 2.6, 3e-4);
        assert_eq!(log.results.len(), 2);
        assert_eq!(log.results[0].step, 1);
        assert!((log.results[0].loss - 2.5).abs() < 1e-6);
        assert!((log.results[1].bpb - 2.6).abs() < 1e-6);
    }

    #[test]
    fn audit_log_to_json_valid() {
        let mut log = AuditLog::new("m", 0, 10, "sha");
        log.record(1, 1.0, 1.5, 3e-4);
        let json = log.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["model_id"], "m");
        assert_eq!(parsed["git_sha"], "sha");
    }

    #[test]
    fn dump_metric_writes_file() {
        let mut log = AuditLog::new("dump-test", 7, 500, "deadbeef");
        log.record(100, 1.2, 1.73, 3e-4);
        log.record(500, 0.8, 1.15, 3e-4);
        let path = "/tmp/trios_test_metric.json";
        log.dump_metric(path).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["model_id"], "dump-test");
        assert_eq!(parsed["completed_step"], 500);
        assert!((parsed["latest_bpb"].as_f64().unwrap() - 1.15f64).abs() < 1e-3);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn dump_metric_empty_results() {
        let log = AuditLog::new("empty", 0, 0, "sha");
        let path = "/tmp/trios_test_metric_empty.json";
        log.dump_metric(path).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["completed_step"], 0);
        std::fs::remove_file(path).ok();
    }
}
