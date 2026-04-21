use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub model_id: String,
    pub seed: u64,
    pub steps: u64,
    pub results: Vec<AuditEntry>,
    pub git_sha: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub step: u64,
    pub loss: f32,
    pub bpb: f32,
    pub lr: f32,
}

impl AuditLog {
    pub fn new(model_id: &str, seed: u64, steps: u64, git_sha: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            seed,
            steps,
            results: Vec::new(),
            git_sha: git_sha.to_string(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn record(&mut self, step: u64, loss: f32, bpb: f32, lr: f32) {
        self.results.push(AuditEntry {
            step,
            loss,
            bpb,
            lr,
        });
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn dump_metric(&self, path: &str) -> anyhow::Result<()> {
        let snapshot = MetricSnapshot {
            model_id: self.model_id.clone(),
            seed: self.seed,
            total_steps: self.steps,
            completed_step: self.results.last().map(|e| e.step).unwrap_or(0),
            latest_loss: self.results.last().map(|e| e.loss).unwrap_or(0.0),
            latest_bpb: self.results.last().map(|e| e.bpb).unwrap_or(0.0),
            git_sha: self.git_sha.clone(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn push_metric(&self, path: &str) -> anyhow::Result<()> {
        self.dump_metric(path)?;
        std::process::Command::new("git")
            .args(["add", path])
            .output()?;
        let msg = format!(
            "metric: step={} bpb={:.4} [bot]",
            self.results.last().map(|e| e.step).unwrap_or(0),
            self.results.last().map(|e| e.bpb).unwrap_or(0.0)
        );
        std::process::Command::new("git")
            .args(["commit", "-m", &msg])
            .output()?;
        std::process::Command::new("git")
            .args(["push", "origin", "HEAD"])
            .output()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub model_id: String,
    pub seed: u64,
    pub total_steps: u64,
    pub completed_step: u64,
    pub latest_loss: f32,
    pub latest_bpb: f32,
    pub git_sha: String,
    pub timestamp: u64,
}
