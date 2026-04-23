//! MCP (Model Context Protocol) JSON-RPC over stdio
//!
//! Handles JSON-RPC 2.0 protocol for MCP stdio communication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::debug;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create new JSON-RPC request with method
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.into(),
            params: None,
        }
    }

    /// Set request ID
    pub fn with_id(mut self, id: Value) -> Self {
        self.id = Some(id);
        self
    }

    /// Set request params
    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create success response
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    /// Create error response
    pub fn error(id: Value, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Parse error from JSON-RPC error code
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create error with data
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Parse error (common error codes)
    pub fn from_code(code: i64) -> Option<Self> {
        match code {
            -32700 => Some(Self::new(code, "Parse error")),
            -32600 => Some(Self::new(code, "Invalid Request")),
            -32601 => Some(Self::new(code, "Method not found")),
            -32602 => Some(Self::new(code, "Invalid params")),
            -32603 => Some(Self::new(code, "Internal error")),
            -32604 => Some(Self::new(code, "Method not found")),
            _ => None,
        }
    }
}

/// MCP Initialize params
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: McpClientCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<McpClientInfo>,
}

/// MCP client capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
}

/// MCP client info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientInfo {
    pub name: String,
    pub version: String,
}

/// MCP server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
}

/// Handle incoming JSON-RPC request and return response
pub fn handle_jsonrpc(
    request: &JsonRpcRequest,
    host: &str,
    port: u16,
) -> Result<JsonRpcResponse> {
    debug!("Received JSON-RPC: {} {}", request.method, request.id);

    match request.method.as_str() {
        "initialize" => {
            let params: InitializeParams = serde_json::from_value(
                request.params.clone().unwrap_or(json!({})))
                .context("Invalid initialize params")?;

            let capabilities = McpServerCapabilities {
                tools: Some(json!(tools::list_tools())),
                resources: None,
                prompts: None,
            };

            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": capabilities,
                "serverInfo": {
                    "name": crate::McpServer::IDENTITY,
                    "version": env!("CARGO_PKG_VERSION"),
                },
            });

            Ok(JsonRpcResponse::success(
                request.id.clone().unwrap_or(json!(null)),
                result,
            ))
        }

        "tools/list" => {
            Ok(JsonRpcResponse::success(
                request.id.clone().unwrap_or(json!(null)),
                json!(tools::list_tools()),
            ))
        }

        "tools/call" => {
            let params = request.params.as_ref().context("Missing tool call params")?;
            let name = params.get("name").and_then(|n| n.as_str()).context("Missing tool name")?;
            let arguments = params.get("arguments").unwrap_or(&json!({}));

            debug!("Calling tool: {} with args: {}", name, arguments);

            match tools::call_tool(name, arguments, host, port).await {
                Ok(result) => Ok(JsonRpcResponse::success(
                    request.id.clone().unwrap_or(json!(null)),
                    result,
                )),
                Err(e) => Ok(JsonRpcResponse::error(
                    request.id.clone().unwrap_or(json!(null)),
                    -32603,
                    format!("Tool execution failed: {}", e),
                )),
            }
        }

        "ping" => {
            Ok(JsonRpcResponse::success(
                request.id.clone().unwrap_or(json!(null)),
                json!({}),
            ))
        }

        _ => {
            Ok(JsonRpcResponse::error(
                request.id.clone().unwrap_or(json!(null)),
                -32601,
                format!("Unknown method: {}", request.method),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_new() {
        let req = JsonRpcRequest::new("tools/list")
            .with_id(json!(1));

        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, Some(json!(1)));
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(json!(1), json!({}));

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let response = JsonRpcResponse::error(json!(1), -32603, "test error");

        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_jsonrpc_error_from_code() {
        assert!(JsonRpcError::from_code(-32601).is_some());
        assert!(JsonRpcError::from_code(-999).is_none());
    }
}
