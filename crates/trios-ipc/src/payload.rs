use serde::{Deserialize, Serialize};
use crate::types::{AgentInfo, McpToolInfo, BrowserCommandReq};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcPayload {
    GetConnectionStatus,
    SendChatMessage {
        text: String,
        agent_id: Option<String>,
    },
    ListAgents,
    ListMcpTools,
    ExecuteBrowserCommand { command: BrowserCommandReq },

    ConnectionStatus {
        connected: bool,
        server_url: String,
        latency_ms: Option<u32>,
    },
    ChatResponse {
        message: String,
        agent_did: String,
        timestamp: u64,
    },
    AgentList { agents: Vec<AgentInfo> },
    McpToolList { tools: Vec<McpToolInfo> },
    BrowserCommandAck {
        command_id: String,
        success: bool,
        error: Option<String>,
    },

    PollBrowserCommands { agent_id: String },
    ReportBrowserResult {
        command_id: String,
        agent_id: String,
        success: bool,
        result: serde_json::Value,
    },

    Error { code: String, message: String },
    Pong { echo_id: String },
}
