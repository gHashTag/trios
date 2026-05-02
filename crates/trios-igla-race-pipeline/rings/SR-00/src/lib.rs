//! SR-00: Scarab Types
//!
//! Core type definitions for IGLA Race pipeline.
//! Dep-free, serde-serializable types extracted from monolith.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Trial identifier (UUID-based wrapper)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::new_without_default)]
pub struct TrialId(pub Uuid);

impl fmt::Display for TrialId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TrialId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl TrialId {
    /// Generate a new random trial ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from string
    pub fn parse(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map_err(|e| anyhow::anyhow!("Invalid trial ID: {}", e))
            .map(Self)
    }
}

/// Hyperparameter configuration for a trial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialConfig {
    /// Architecture name
    pub arch: String,
    /// Hidden layer size
    #[serde(rename = "d_model")]
    pub hidden: usize,
    /// N-gram context size
    #[serde(rename = "n_gram")]
    pub context: usize,
    /// Learning rate
    pub lr: f64,
    /// Random seed
    pub seed: i64,
    /// Optimizer name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimizer: Option<String>,
    /// Weight decay (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wd: Option<f64>,
    /// Activation function (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation: Option<String>,
}

impl PartialEq for TrialConfig {
    fn eq(&self, other: &Self) -> bool {
        self.arch == other.arch
            && self.hidden == other.hidden
            && self.context == other.context
            && (self.lr - other.lr).abs() < f64::EPSILON
            && self.seed == other.seed
            && self.optimizer == other.optimizer
            && self.wd == other.wd
            && self.activation == other.activation
    }
}

impl TrialConfig {
    /// Create a minimal config for testing
    pub fn minimal() -> Self {
        Self {
            arch: "ngram".to_string(),
            hidden: 384,
            context: 6,
            lr: 0.004,
            seed: 42,
            optimizer: None,
            wd: None,
            activation: None,
        }
    }

    /// Validate config constraints
    pub fn validate(&self) -> Result<()> {
        if self.hidden == 0 {
            return Err(anyhow::anyhow!("hidden layer size must be > 0"));
        }
        if self.lr <= 0.0 {
            return Err(anyhow::anyhow!("learning rate must be > 0"));
        }
        if self.context == 0 {
            return Err(anyhow::anyhow!("context size must be > 0"));
        }
        Ok(())
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))
    }

    /// Deserialize from JSON
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize config: {}", e))
    }
}

/// Trial lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Trial is currently running
    #[serde(rename = "running")]
    Running,
    /// Trial was pruned at a rung
    #[serde(rename = "pruned")]
    Pruned,
    /// Trial completed normally
    #[serde(rename = "complete")]
    Complete,
    /// IGLA criterion satisfied (victory)
    #[serde(rename = "igla_found")]
    IGLAFound,
    /// Trial encountered an error
    #[serde(rename = "error")]
    Error,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Running => write!(f, "running"),
            JobStatus::Pruned => write!(f, "pruned"),
            JobStatus::Complete => write!(f, "complete"),
            JobStatus::IGLAFound => write!(f, "igla_found"),
            JobStatus::Error => write!(f, "error"),
        }
    }
}

impl JobStatus {
    /// Check if status is terminal (no more changes)
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Complete | JobStatus::IGLAFound | JobStatus::Error)
    }

    /// Check if status allows pruning
    pub fn can_prune(&self) -> bool {
        matches!(self, JobStatus::Running)
    }
}

/// Result from a single ASHA rung
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RungResult {
    /// Rung number (1-4)
    pub rung: u8,
    /// BPB at this rung
    pub bpb: f64,
    /// Timestamp of measurement
    pub timestamp: DateTime<Utc>,
    /// Number of steps at this rung
    pub steps: usize,
}

impl RungResult {
    /// Create a new rung result
    pub fn new(rung: u8, bpb: f64, steps: usize) -> Self {
        Self {
            rung,
            bpb,
            timestamp: Utc::now(),
            steps,
        }
    }
}

impl PartialEq for RungResult {
    fn eq(&self, other: &Self) -> bool {
        self.rung == other.rung
            && (self.bpb - other.bpb).abs() < f64::EPSILON
    }
}

/// Complete trial record with all rung results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialRecord {
    /// Trial ID
    pub trial_id: TrialId,
    /// Machine identifier
    pub machine_id: String,
    /// Worker identifier
    pub worker_id: String,
    /// Current status
    pub status: JobStatus,
    /// Trial configuration
    pub config: TrialConfig,
    /// Rung 1 result (1000 steps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rung_1: Option<RungResult>,
    /// Rung 2 result (3000 steps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rung_2: Option<RungResult>,
    /// Rung 3 result (9000 steps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rung_3: Option<RungResult>,
    /// Rung 4 result (27000 steps)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rung_4: Option<RungResult>,
    /// Best BPB across all rungs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_bpb: Option<f64>,
    /// Step count at best BPB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_step: Option<usize>,
    /// Timestamp when trial was pruned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pruned_at_step: Option<usize>,
    /// Timestamp when trial completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// When trial was created
    pub created_at: DateTime<Utc>,
}

impl TrialRecord {
    /// Create a new trial record
    pub fn new(trial_id: TrialId, machine_id: String, worker_id: String, config: TrialConfig) -> Self {
        Self {
            trial_id,
            machine_id,
            worker_id,
            status: JobStatus::Running,
            config,
            rung_1: None,
            rung_2: None,
            rung_3: None,
            rung_4: None,
            best_bpb: None,
            best_step: None,
            pruned_at_step: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    /// Update a rung result
    pub fn update_rung(&mut self, rung: u8, result: RungResult) {
        match rung {
            1 => self.rung_1 = Some(result),
            2 => self.rung_2 = Some(result),
            3 => self.rung_3 = Some(result),
            4 => self.rung_4 = Some(result),
            _ => {}
        }
    }

    /// Get rung reference (borrows the result)
    pub fn get_rung(&self, rung: u8) -> Option<&RungResult> {
        match rung {
            1 => self.rung_1.as_ref(),
            2 => self.rung_2.as_ref(),
            3 => self.rung_3.as_ref(),
            4 => self.rung_4.as_ref(),
            _ => None,
        }
    }

    /// Update best BPB if this is better
    pub fn update_best_bpb(&mut self, bpb: f64, step: usize) {
        let current_best = self.best_bpb.unwrap_or(f64::INFINITY);
        if current_best > bpb {
            self.best_bpb = Some(bpb);
            self.best_step = Some(step);
        }
    }

    /// Mark trial as pruned
    pub fn mark_pruned(&mut self, at_step: usize) {
        self.status = JobStatus::Pruned;
        self.pruned_at_step = Some(at_step);
    }

    /// Mark trial as complete with final result
    pub fn mark_complete(&mut self, status: JobStatus) {
        self.status = status;
        self.completed_at = Some(Utc::now());
    }
}

/// Failure memory protocol entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEntry {
    /// Associated trial ID
    pub trial_id: TrialId,
    /// Outcome (pruned, complete, igla_found, error)
    pub outcome: JobStatus,
    /// Rung at which trial was pruned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pruned_at_rung: Option<u8>,
    /// BPB when pruned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pruned_bpb: Option<f64>,
    /// Reason for pruning (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pruned_reason: Option<String>,
    /// Auto-generated lesson
    pub lesson: String,
    /// Lesson type (WARN, PATTERN, SUCCESS, TIP)
    pub lesson_type: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Pattern key for failure grouping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_key: Option<String>,
    /// How many times this pattern failed
    pub pattern_count: i32,
    /// When this experience was recorded
    pub created_at: DateTime<Utc>,
}

impl ExperienceEntry {
    /// Create a new experience entry
    pub fn new(trial_id: TrialId, outcome: JobStatus) -> Self {
        Self {
            trial_id,
            outcome,
            pruned_at_rung: None,
            pruned_bpb: None,
            pruned_reason: None,
            lesson: String::new(),
            lesson_type: "INFO".to_string(),
            confidence: 1.0,
            pattern_key: None,
            pattern_count: 1,
            created_at: Utc::now(),
        }
    }

    /// Mark as pruning event
    pub fn mark_pruned(&mut self, rung: u8, bpb: f64, reason: String) {
        self.outcome = JobStatus::Pruned;
        self.pruned_at_rung = Some(rung);
        self.pruned_bpb = Some(bpb);
        self.pruned_reason = Some(reason);
        self.lesson_type = "PATTERN".to_string();
    }

    /// Mark as success
    pub fn mark_success(&mut self, lesson: String) {
        self.outcome = JobStatus::IGLAFound;
        self.lesson = lesson;
        self.lesson_type = "SUCCESS".to_string();
        self.confidence = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trial_id_generation() {
        let id1 = TrialId::new();
        let id2 = TrialId::new();
        assert_ne!(id1, id2);

        let parsed = TrialId::parse(&id1.to_string()).unwrap();
        assert_eq!(parsed, id1);
    }

    #[test]
    fn test_trial_config_validation() {
        let valid_config = TrialConfig::minimal();
        assert!(valid_config.validate().is_ok());

        let invalid_config = TrialConfig {
            hidden: 0,
            ..TrialConfig::minimal()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_trial_config_json() {
        let config = TrialConfig::minimal();
        let json = config.to_json().unwrap();
        let restored = TrialConfig::from_json(&json).unwrap();
        assert_eq!(restored.arch, config.arch);
        assert_eq!(restored.lr, config.lr);
    }

    #[test]
    fn test_job_status_display() {
        assert_eq!(format!("{}", JobStatus::Running), "running");
        assert_eq!(format!("{}", JobStatus::IGLAFound), "igla_found");
    }

    #[test]
    fn test_job_status_is_terminal() {
        assert!(JobStatus::Complete.is_terminal());
        assert!(JobStatus::IGLAFound.is_terminal());
        assert!(JobStatus::Error.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(!JobStatus::Pruned.is_terminal());
    }

    #[test]
    fn test_rung_result_creation() {
        let result = RungResult::new(2, 1.23, 1500);
        assert_eq!(result.rung, 2);
        assert_eq!(result.bpb, 1.23);
        assert_eq!(result.steps, 1500);
    }

    #[test]
    fn test_trial_record_rung_update() {
        let trial_id = TrialId::new();
        let mut record = TrialRecord::new(trial_id, "machine-1".to_string(), "worker-0".to_string(), TrialConfig::minimal());

        let result = RungResult::new(1, 1.5, 1000);
        record.update_rung(1, result.clone());
        assert_eq!(record.get_rung(1), Some(&result));

        let result2 = RungResult::new(2, 1.4, 3000);
        record.update_rung(2, result2.clone());
        assert_eq!(record.get_rung(2), Some(&result2));
    }

    #[test]
    fn test_trial_record_best_bpb() {
        let trial_id = TrialId::new();
        let mut record = TrialRecord::new(trial_id, "machine-1".to_string(), "worker-0".to_string(), TrialConfig::minimal());

        // First update
        record.update_best_bpb(2.0, 1000);
        assert_eq!(record.best_bpb, Some(2.0));
        assert_eq!(record.best_step, Some(1000));

        // Better value
        record.update_best_bpb(1.8, 2000);
        assert_eq!(record.best_bpb, Some(1.8));
        assert_eq!(record.best_step, Some(2000));

        // Worse value (should not update)
        record.update_best_bpb(2.5, 3000);
        assert_eq!(record.best_bpb, Some(1.8));
        assert_eq!(record.best_step, Some(2000));
    }

    #[test]
    fn test_trial_record_marking() {
        let trial_id = TrialId::new();
        let mut record = TrialRecord::new(trial_id, "machine-1".to_string(), "worker-0".to_string(), TrialConfig::minimal());

        assert_eq!(record.status, JobStatus::Running);

        record.mark_pruned(500);
        assert_eq!(record.status, JobStatus::Pruned);
        assert_eq!(record.pruned_at_step, Some(500));

        record.mark_complete(JobStatus::IGLAFound);
        assert_eq!(record.status, JobStatus::IGLAFound);
        assert!(record.completed_at.is_some());
    }

    #[test]
    fn test_experience_entry() {
        let trial_id = TrialId::new();
        let mut entry = ExperienceEntry::new(trial_id, JobStatus::Running);

        entry.mark_pruned(2, 2.5, "LR too high".to_string());
        assert_eq!(entry.outcome, JobStatus::Pruned);
        assert_eq!(entry.pruned_at_rung, Some(2));
        assert_eq!(entry.pruned_bpb, Some(2.5));
        assert_eq!(entry.pruned_reason, Some("LR too high".to_string()));
        assert_eq!(entry.lesson_type, "PATTERN");

        entry.mark_success("Great config!".to_string());
        assert_eq!(entry.outcome, JobStatus::IGLAFound);
        assert_eq!(entry.lesson_type, "SUCCESS");
    }
}
