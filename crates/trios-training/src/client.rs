//! HTTP client for the trinity-training Railway API.

use crate::types::*;
use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, instrument};

/// REST client for the trinity-training API.
#[derive(Debug, Clone)]
pub struct TrainingClient {
    base_url: String,
    client: Client,
}

impl TrainingClient {
    /// Create a new training client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Create a client for Railway deployment.
    pub fn railway() -> Self {
        Self::new("https://trinity-training.up.railway.app")
    }

    /// Create a client for local development.
    pub fn localhost() -> Self {
        Self::new("http://localhost:8082")
    }

    // ── Job lifecycle ──────────────────────────────────────────

    /// Start a new training job.
    #[instrument(skip(self, config))]
    pub async fn start_job(
        &self,
        name: &str,
        script: &str,
        config: &JobConfig,
    ) -> Result<JobStatusResponse> {
        let body = serde_json::json!({
            "name": name,
            "script": script,
            "config": config,
        });
        self.post("/api/jobs", &body).await
    }

    /// Get the status of a training job.
    #[instrument(skip(self))]
    pub async fn job_status(&self, job_id: &str) -> Result<JobStatusResponse> {
        self.get(&format!("/api/jobs/{job_id}")).await
    }

    /// Cancel a running training job.
    #[instrument(skip(self))]
    pub async fn cancel_job(&self, job_id: &str) -> Result<()> {
        self.client
            .post(format!("{}/api/jobs/{job_id}/cancel", self.base_url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// List all training jobs.
    #[instrument(skip(self))]
    pub async fn list_jobs(&self) -> Result<Vec<JobStatusResponse>> {
        self.get("/api/jobs").await
    }

    /// Get the result of a completed training job.
    #[instrument(skip(self))]
    pub async fn job_result(&self, job_id: &str) -> Result<TrainingResult> {
        self.get(&format!("/api/jobs/{job_id}/result")).await
    }

    // ── Logs ───────────────────────────────────────────────────

    /// Get training logs for a job.
    #[instrument(skip(self))]
    pub async fn job_logs(&self, job_id: &str) -> Result<Vec<JobLogEntry>> {
        self.get(&format!("/api/jobs/{job_id}/logs")).await
    }

    /// Stream training logs (returns recent logs, use polling for live updates).
    #[instrument(skip(self))]
    pub async fn recent_logs(&self, job_id: &str, limit: usize) -> Result<Vec<JobLogEntry>> {
        self.get(&format!("/api/jobs/{job_id}/logs?limit={limit}"))
            .await
    }

    // ── Artifacts ──────────────────────────────────────────────

    /// Download the trained model artifact.
    #[instrument(skip(self))]
    pub async fn download_artifact(&self, job_id: &str) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get(format!("{}/api/jobs/{job_id}/artifact", self.base_url))
            .send()
            .await?;
        resp.error_for_status_ref()?;
        Ok(resp.bytes().await?.to_vec())
    }

    // ── Health ─────────────────────────────────────────────────

    /// Check if the training server is healthy.
    pub async fn health(&self) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await;
        match resp {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    // ── Internal helpers ───────────────────────────────────────

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        debug!("GET {}{}", self.base_url, path);
        let resp = self
            .client
            .get(format!("{}{path}", self.base_url))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Training API error {status}: {text}");
        }
        Ok(resp.json().await?)
    }

    async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        debug!("POST {}{}", self.base_url, path);
        let resp = self
            .client
            .post(format!("{}{path}", self.base_url))
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Training API error {status}: {text}");
        }
        Ok(resp.json().await?)
    }
}
