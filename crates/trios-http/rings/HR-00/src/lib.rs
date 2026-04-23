//! HR-00 — Core types for trios-http
//! Ring 00 — identity and request/response structs

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state
#[derive(Clone, Debug)]
pub struct AppState {
    pub agents: Arc<RwLock<u32>>,
    pub tools: u32,
}

impl AppState {
    pub fn new(tools: u32) -> Self {
        Self {
            agents: Arc::new(RwLock::new(0)),
            tools,
        }
    }
}

/// POST /api/chat request body
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatRequest {
    pub method: String,
    pub params: serde_json::Value,
}

/// POST /api/chat response body
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GET /api/status response body
#[derive(Debug, Serialize, Clone)]
pub struct StatusResponse {
    pub status: &'static str,
    pub agents: u32,
    pub tools: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new(19);
        assert_eq!(state.tools, 19);
    }

    #[test]
    fn test_chat_request_deserialize() {
        let json = r##"{"method":"agents/list","params":{}}"##;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "agents/list");
    }

    #[test]
    fn test_status_response_serialize() {
        let resp = StatusResponse { status: "ok", agents: 0, tools: 19 };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"tools\":19"));
    }
}
