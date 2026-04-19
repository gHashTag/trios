//! # trios-agents
//!
//! HTTP client and MCP proxy for [zig-agents](https://github.com/gHashTag/zig-agents),
//! providing AI agent orchestration capabilities.
//!
//! ## Example
//!
//! ```ignore
//! use trios_agents::AgentsClient;
//!
//! let client = AgentsClient::localhost();
//! let agent = client.spawn("researcher", "Analyze GF16 compression").await?;
//! let status = client.status(&agent.id).await?;
//! ```

mod client;
mod types;

pub use client::AgentsClient;
pub use types::{
    AgentConfig, AgentId, AgentMessage, AgentStatus, AgentTask, SpawnRequest, TaskResult,
};
