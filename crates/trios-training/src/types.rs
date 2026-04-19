//! Types for the trinity-training API.

use serde::{Deserialize, Serialize};

/// Unique training job identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobId(pub String);

/// GPU type selection for training jobs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GpuType {
    H100,
    A100,
    A6000,
    A10G,
    T4,
}

/// Configuration for a training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Number of GPUs to use.
    pub num_gpus: usize,
    /// GPU type.
    pub gpu_type: GpuType,
    /// Maximum training time in minutes.
    pub max_time_minutes: u64,
    /// Model configuration as JSON.
    pub model_config: serde_json::Value,
    /// Training hyperparameters.
    pub hyperparams: Hyperparams,
    /// Dataset configuration.
    pub dataset: DatasetConfig,
}

/// Training hyperparameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperparams {
    /// Learning rate.
    pub learning_rate: f64,
    /// Batch size.
    pub batch_size: usize,
    /// Number of epochs/iterations.
    pub iterations: usize,
    /// Optimizer type (e.g., "adamw", "muon").
    pub optimizer: String,
    /// Weight decay.
    #[serde(default)]
    pub weight_decay: f64,
    /// Warmup steps.
    #[serde(default)]
    pub warmup_steps: usize,
    /// Learning rate schedule (e.g., "cosine", "phi_decay").
    #[serde(default = "default_schedule")]
    pub schedule: String,
    /// EMA decay rate.
    #[serde(default)]
    pub ema_decay: Option<f64>,
}

fn default_schedule() -> String {
    "cosine".to_string()
}

/// Dataset configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Dataset name (e.g., "fineweb").
    pub name: String,
    /// Number of documents to use.
    pub num_docs: Option<usize>,
    /// Sequence length.
    pub seq_len: usize,
    /// Vocab size.
    pub vocab_size: usize,
}

/// Current status of a training job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Response from job status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    /// Job ID.
    pub id: String,
    /// Job name.
    pub name: String,
    /// Current status.
    pub status: JobStatus,
    /// Progress percentage (0-100).
    pub progress: f64,
    /// Current training metrics.
    pub metrics: Option<JobMetrics>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Completion timestamp.
    pub completed_at: Option<String>,
    /// GPU hours used.
    pub gpu_hours: f64,
}

/// Training metrics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    /// Current loss.
    pub loss: f64,
    /// Bits per byte (BPB) on validation.
    pub bpb: Option<f64>,
    /// Learning rate.
    pub learning_rate: f64,
    /// Steps completed.
    pub step: usize,
    /// Total steps.
    pub total_steps: usize,
    /// Tokens processed.
    pub tokens_processed: u64,
    /// Throughput in tokens/second.
    pub throughput: f64,
    /// GPU memory usage in GB.
    pub gpu_memory_gb: f64,
}

/// Result of a completed training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    /// Job ID.
    pub job_id: String,
    /// Final BPB score.
    pub final_bpb: f64,
    /// Final loss.
    pub final_loss: f64,
    /// Total training time in seconds.
    pub training_time_secs: f64,
    /// Total tokens processed.
    pub total_tokens: u64,
    /// Artifact size in bytes.
    pub artifact_size_bytes: u64,
    /// Download URL for the trained model artifact.
    pub artifact_url: String,
    /// Training log entries.
    pub log: Vec<JobLogEntry>,
}

/// A single log entry from a training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobLogEntry {
    /// Timestamp.
    pub timestamp: String,
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
}
