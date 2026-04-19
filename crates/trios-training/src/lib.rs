//! # trios-training
//!
//! HTTP client for the [trinity-training](https://github.com/gHashTag/trinity-training)
//! Railway API — training job orchestration for AI models.
//!
//! ## Example
//!
//! ```ignore
//! use trios_training::TrainingClient;
//!
//! let client = TrainingClient::new("https://trinity-training.up.railway.app");
//! let job = client.start_job("gf16-experiment", "train_gpt.py", &config).await?;
//! let status = client.job_status(&job.id).await?;
//! ```

mod client;
mod types;

pub use client::TrainingClient;
pub use types::{
    GpuType, JobConfig, JobId, JobLogEntry, JobMetrics, JobStatus, JobStatusResponse,
    TrainingResult,
};
