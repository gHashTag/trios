//! HTTP client for the zig-agents REST API.

use crate::types::*;
use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, instrument};

/// Default base URL for the agents server.
pub const DEFAULT_AGENTS_URL: &str = "http://localhost:8081";

/// REST client for the zig-agents API.
#[derive(Debug, Clone)]
pub struct AgentsClient {
    base_url: String,
    client: Client,
}

impl AgentsClient {
    /// Create a new agents client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Create a client with default URL.
    pub fn localhost() -> Self {
        Self::new(DEFAULT_AGENTS_URL)
    }

    // ── Agent lifecycle ────────────────────────────────────────

    /// Spawn a new agent with an initial task.
    #[instrument(skip(self))]
    pub async fn spawn(&self, agent_type: &str, task: &str) -> Result<AgentStatus> {
        let body = SpawnRequest {
            agent_type: agent_type.to_string(),
            task: task.to_string(),
            config: None,
            context: serde_json::Value::Null,
        };
        self.post("/api/agents", &body).await
    }

    /// Spawn an agent with full configuration.
    #[instrument(skip(self))]
    pub async fn spawn_with_config(&self, request: &SpawnRequest) -> Result<AgentStatus> {
        self.post("/api/agents", request).await
    }

    /// Get agent status.
    #[instrument(skip(self))]
    pub async fn status(&self, agent_id: &str) -> Result<AgentStatus> {
        self.get(&format!("/api/agents/{agent_id}")).await
    }

    /// Stop an agent.
    #[instrument(skip(self))]
    pub async fn stop(&self, agent_id: &str) -> Result<()> {
        self.client
            .post(format!("{}/api/agents/{agent_id}/stop", self.base_url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// List all active agents.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<AgentStatus>> {
        self.get("/api/agents").await
    }

    // ── Task management ────────────────────────────────────────

    /// Assign a new task to an agent.
    #[instrument(skip(self))]
    pub async fn assign_task(&self, agent_id: &str, description: &str) -> Result<AgentTask> {
        let body = serde_json::json!({ "description": description });
        self.post(&format!("/api/agents/{agent_id}/tasks"), &body).await
    }

    /// Get task status.
    #[instrument(skip(self))]
    pub async fn task_status(&self, agent_id: &str, task_id: &str) -> Result<AgentTask> {
        self.get(&format!("/api/agents/{agent_id}/tasks/{task_id}")).await
    }

    /// Cancel a task.
    #[instrument(skip(self))]
    pub async fn cancel_task(&self, agent_id: &str, task_id: &str) -> Result<()> {
        self.client
            .post(format!(
                "{}/api/agents/{agent_id}/tasks/{task_id}/cancel",
                self.base_url
            ))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    // ── Messaging ──────────────────────────────────────────────

    /// Send a message to an agent.
    #[instrument(skip(self))]
    pub async fn send_message(&self, agent_id: &str, content: &str) -> Result<AgentMessage> {
        let body = serde_json::json!({
            "role": "user",
            "content": content
        });
        self.post(&format!("/api/agents/{agent_id}/messages"), &body).await
    }

    /// Get conversation history for an agent.
    #[instrument(skip(self))]
    pub async fn get_messages(&self, agent_id: &str) -> Result<Vec<AgentMessage>> {
        self.get(&format!("/api/agents/{agent_id}/messages")).await
    }

    // ── Health ─────────────────────────────────────────────────

    /// Check if the agents server is healthy.
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
            anyhow::bail!("Agents API error {status}: {text}");
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
            anyhow::bail!("Agents API error {status}: {text}");
        }
        Ok(resp.json().await?)
    }
}
