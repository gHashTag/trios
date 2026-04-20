use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

use std::collections::HashMap;
use std::sync::Arc;

use crate::process::{AgentId, AgentStatus, ChildProcess};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub agent_id: AgentId,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    pub model: String,
    pub name: String,
    pub max_tokens: Option<u32>,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            model: "claude-opus-4-5".to_string(),
            name: String::new(),
            max_tokens: None,
        }
    }
}

#[derive(Default)]
pub struct ClaudeBridge {
    processes: Arc<Mutex<HashMap<AgentId, ChildProcess>>>,
}

impl ClaudeBridge {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn_agent(&self, config: SpawnConfig) -> anyhow::Result<AgentId> {
        let id = AgentId(format!("agent-{}", Uuid::new_v4()));
        let name = if config.name.is_empty() {
            id.0.clone()
        } else {
            config.name
        };

        info!("Spawning agent: name={}, model={}", name, config.model);

        let child = Command::new("claude")
            .arg("--model")
            .arg(&config.model)
            .arg("--print")
            .arg("ready")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(process) => {
                let entry = ChildProcess {
                    id: id.clone(),
                    name,
                    model: config.model,
                    status: AgentStatus::Ready,
                    pid: process.id(),
                };

                self.processes.lock().await.insert(id.clone(), entry);
                info!("Agent spawned: {}", id.0);
                Ok(id)
            }
            Err(e) => {
                error!("Failed to spawn claude process: {}", e);
                Err(anyhow::anyhow!("failed to spawn claude: {}", e))
            }
        }
    }

    pub async fn send_task(&self, id: &AgentId, task: &str) -> anyhow::Result<TaskResult> {
        let mut processes = self.processes.lock().await;
        let entry = processes
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("agent {} not found", id.0))?;

        entry.status = AgentStatus::Working;
        let model = entry.model.clone();
        drop(processes);

        info!("Sending task to agent {}: {} chars", id.0, task.len());

        let mut cmd = Command::new("claude");
        cmd.arg("--model").arg(&model).arg("--print").arg(task);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().await.map_err(|e| {
            let mut processes = self.processes.blocking_lock();
            if let Some(entry) = processes.get_mut(id) {
                entry.status = AgentStatus::Error(e.to_string());
            }
            anyhow::anyhow!("claude process failed: {}", e)
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut processes = self.processes.lock().await;
        let entry = processes.get_mut(id).ok_or_else(|| anyhow::anyhow!("agent lost"))?;

        if output.status.success() {
            entry.status = AgentStatus::Idle;
            Ok(TaskResult {
                agent_id: id.clone(),
                output: stdout,
                success: true,
            })
        } else {
            entry.status = AgentStatus::Error(stderr.clone());
            Ok(TaskResult {
                agent_id: id.clone(),
                output: stderr,
                success: false,
            })
        }
    }

    pub async fn get_status(&self, id: &AgentId) -> anyhow::Result<AgentStatus> {
        let processes = self.processes.lock().await;
        let entry = processes
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("agent {} not found", id.0))?;
        Ok(entry.status.clone())
    }

    pub async fn list_agents(&self) -> Vec<ChildProcess> {
        let processes = self.processes.lock().await;
        processes.values().cloned().collect()
    }

    pub async fn kill_agent(&self, id: &AgentId) -> anyhow::Result<()> {
        let mut processes = self.processes.lock().await;
        if let Some(mut entry) = processes.remove(id) {
            entry.status = AgentStatus::Terminated;
            info!("Agent {} terminated", id.0);
        }
        Ok(())
    }

    pub async fn send_task_streaming(
        &self,
        id: &AgentId,
        task: &str,
        mut on_chunk: impl FnMut(&str),
    ) -> anyhow::Result<TaskResult> {
        let mut processes = self.processes.lock().await;
        let entry = processes
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("agent {} not found", id.0))?;

        entry.status = AgentStatus::Working;
        let model = entry.model.clone();
        drop(processes);

        let mut child = Command::new("claude")
            .arg("--model")
            .arg(&model)
            .arg("--print")
            .arg(task)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take();
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                on_chunk(&line);
            }
        }

        let status = child.wait().await?;
        let mut processes = self.processes.lock().await;
        if let Some(entry) = processes.get_mut(id) {
            entry.status = if status.success() {
                AgentStatus::Idle
            } else {
                AgentStatus::Error("non-zero exit".into())
            };
        }

        Ok(TaskResult {
            agent_id: id.clone(),
            output: String::new(),
            success: status.success(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_new() {
        let bridge = ClaudeBridge::new();
        let agents = bridge.list_agents().await;
        assert!(agents.is_empty());
    }

    #[tokio::test]
    async fn test_get_status_nonexistent() {
        let bridge = ClaudeBridge::new();
        let result = bridge.get_status(&AgentId("nonexistent".into())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_kill_nonexistent() {
        let bridge = ClaudeBridge::new();
        let result = bridge.kill_agent(&AgentId("ghost".into())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_spawn_config_default() {
        let config = SpawnConfig::default();
        assert_eq!(config.model, "claude-opus-4-5");
        assert!(config.max_tokens.is_none());
    }

    #[tokio::test]
    async fn test_list_empty() {
        let bridge = ClaudeBridge::new();
        assert_eq!(bridge.list_agents().await.len(), 0);
    }

    #[tokio::test]
    async fn test_send_task_nonexistent() {
        let bridge = ClaudeBridge::new();
        let result = bridge.send_task(&AgentId("nobody".into()), "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_id_display() {
        let id = AgentId("agent-test".into());
        assert!(id.0.contains("agent-test"));
    }
}
