//! Types for the zig-agents API.

use serde::{Deserialize, Serialize};

/// Unique agent identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentId(pub String);

/// Configuration for spawning a new agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent type (e.g., "researcher", "coder", "analyst").
    pub agent_type: String,
    /// Maximum memory in MB.
    #[serde(default = "default_memory")]
    pub max_memory_mb: usize,
    /// Maximum concurrent tasks.
    #[serde(default = "default_concurrency")]
    pub max_concurrency: usize,
    /// Timeout per task in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_memory() -> usize {
    512
}
fn default_concurrency() -> usize {
    4
}
fn default_timeout() -> u64 {
    300
}

/// Request to spawn a new agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Agent type to spawn.
    pub agent_type: String,
    /// Initial task description.
    pub task: String,
    /// Optional configuration overrides.
    #[serde(default)]
    pub config: Option<AgentConfig>,
    /// Context data to pass to the agent.
    #[serde(default)]
    pub context: serde_json::Value,
}

/// Status of an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    /// Agent ID.
    pub id: String,
    /// Agent type.
    pub agent_type: String,
    /// Current state.
    pub state: AgentState,
    /// Number of completed tasks.
    pub tasks_completed: usize,
    /// Number of pending tasks.
    pub tasks_pending: usize,
    /// Uptime in seconds.
    pub uptime_secs: f64,
    /// Memory usage in MB.
    pub memory_mb: f64,
}

/// Agent lifecycle states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Starting,
    Idle,
    Running,
    Waiting,
    Stopping,
    Stopped,
    Error,
}

/// A task assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Task ID.
    pub id: String,
    /// Agent ID this task is assigned to.
    pub agent_id: String,
    /// Task description.
    pub description: String,
    /// Task state.
    pub state: TaskState,
    /// Result (if completed).
    pub result: Option<TaskResult>,
}

/// Task lifecycle states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Result of a completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Output text.
    pub output: String,
    /// Structured result data.
    pub data: serde_json::Value,
    /// Execution time in seconds.
    pub duration_secs: f64,
    /// Whether the task succeeded.
    pub success: bool,
}

/// A message sent to/from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message role (user, agent, system).
    pub role: String,
    /// Message content.
    pub content: String,
    /// Timestamp.
    pub timestamp: Option<String>,
}
