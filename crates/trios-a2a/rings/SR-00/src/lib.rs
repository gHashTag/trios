//! SR-00 — Agent Identity
//!
//! Core types for agent identification and capability declaration.
//! Every agent in the TRIOS ecosystem has an AgentId and an AgentCard.

use serde::{Deserialize, Serialize};

/// Unique agent identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent capability — what an agent can do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Capability {
    /// Can execute code (build, test, lint)
    Codegen,
    /// Can read/write files
    FileSystem,
    /// Can interact with git
    Git,
    /// Can run shell commands
    Shell,
    /// Can call LLM APIs
    LLM,
    /// Can manage other agents
    Orchestrator,
    /// Custom capability
    Custom(String),
}

/// Agent card — identity + capabilities + status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub status: AgentStatus,
    pub description: String,
}

/// Agent lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is available for tasks
    Idle,
    /// Agent is working on a task
    Busy,
    /// Agent is disconnected
    Offline,
    /// Agent encountered an error
    Error,
}

impl AgentCard {
    /// Create a new agent card.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(id),
            name: name.into(),
            capabilities: Vec::new(),
            status: AgentStatus::Idle,
            description: String::new(),
        }
    }

    /// Add a capability.
    pub fn with_capability(mut self, cap: Capability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Check if agent has a specific capability.
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    /// Check if agent is available for work.
    pub fn is_available(&self) -> bool {
        self.status == AgentStatus::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new("alpha-1");
        assert_eq!(id.to_string(), "alpha-1");
    }

    #[test]
    fn test_agent_card_builder() {
        let card = AgentCard::new("alpha-1", "Alpha")
            .with_capability(Capability::Codegen)
            .with_capability(Capability::Git);
        assert_eq!(card.capabilities.len(), 2);
        assert!(card.has_capability(&Capability::Codegen));
        assert!(!card.has_capability(&Capability::LLM));
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus::Busy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"busy\"");
    }

    #[test]
    fn test_agent_is_available() {
        let mut card = AgentCard::new("alpha-1", "Alpha");
        assert!(card.is_available());
        card.status = AgentStatus::Busy;
        assert!(!card.is_available());
    }
}
