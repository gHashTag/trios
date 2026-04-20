use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Ready,
    Idle,
    Working,
    Error(String),
    Terminated,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Ready => write!(f, "ready"),
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Working => write!(f, "working"),
            AgentStatus::Error(e) => write!(f, "error: {}", e),
            AgentStatus::Terminated => write!(f, "terminated"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildProcess {
    pub id: AgentId,
    pub name: String,
    pub model: String,
    pub status: AgentStatus,
    pub pid: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_status_display() {
        assert_eq!(format!("{}", AgentStatus::Ready), "ready");
        assert_eq!(format!("{}", AgentStatus::Idle), "idle");
        assert_eq!(format!("{}", AgentStatus::Working), "working");
        assert_eq!(format!("{}", AgentStatus::Terminated), "terminated");
        assert_eq!(
            format!("{}", AgentStatus::Error("crash".into())),
            "error: crash"
        );
    }

    #[test]
    fn agent_id_display() {
        let id = AgentId("agent-abc".into());
        assert_eq!(format!("{}", id), "agent-abc");
    }

    #[test]
    fn agent_status_serialize() {
        let status = AgentStatus::Working;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("working"));
    }
}
