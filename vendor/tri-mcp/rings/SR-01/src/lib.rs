//! SR-01 — MCP Protocol Types (Gold, Level 01)
//!
//! JSON-RPC 2.0 + Model Context Protocol types.
//! Based on MCP 2024-11-05 spec.
//! No dependencies on other rings — pure types.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ============================================================================
// JSON-RPC 2.0 Base Types
// ============================================================================

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
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
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ============================================================================
// MCP Protocol Types
// ============================================================================

/// Initialize request params
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
}

/// Initialize response result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Client capabilities
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RootsCapability {
    pub list_changed: bool,
}

/// Server capabilities
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingCapability {
    pub level: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PromptsCapability {
    pub list_changed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourcesCapability {
    pub subscribe: bool,
    pub list_changed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsCapability {
    pub list_changed: bool,
}

/// Client info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Server info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// ============================================================================
// Tool Types
// ============================================================================

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Tools/list result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

/// Tool call request
#[derive(Debug, Clone, Deserialize)]
pub struct CallToolRequest {
    pub name: String,
    pub arguments: Value,
}

/// Tool call result
#[derive(Debug, Clone, Serialize)]
pub struct CallToolResult {
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

/// Content block in tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { uri: String, mime_type: Option<String>, text: Option<String> },
}

// ============================================================================
// Browser Event Types (for Chrome Extension → Server communication)
// ============================================================================

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Browser log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserLog {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: i64,
    pub source: Option<String>,
    pub url: Option<String>,
}

/// Network request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub timestamp: i64,
    pub duration: Option<u64>,
    pub size: Option<u64>,
}

/// Screenshot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotData {
    pub data: String, // base64
    pub timestamp: i64,
    pub width: u32,
    pub height: u32,
}

/// DOM element info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomElement {
    pub tag_name: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub text: Option<String>,
    pub attributes: Vec<(String, String)>,
    pub xpath: Option<String>,
}

/// Selected element (alias for extension communication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedElement {
    pub tag_name: String,
    pub id: Option<String>,
    pub class_name: Option<String>,
    pub attributes: serde_json::Value,
}

// ============================================================================
// WebSocket Messages from Chrome Extension (browser-connector.ts wss.on("message"))
// ============================================================================

/// Extension message types (WS protocol between Chrome Extension and Server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtensionMessage {
    #[serde(rename = "screenshot-data")]
    ScreenshotData { data: String, path: Option<String>, auto_paste: Option<bool> },
    #[serde(rename = "screenshot-error")]
    ScreenshotError { error: String },
    #[serde(rename = "current-url-response")]
    UrlResponse { url: String, tab_id: Option<serde_json::Value>, request_id: Option<String> },
    #[serde(rename = "page-navigated")]
    PageNavigated { url: String, tab_id: Option<serde_json::Value> },
    #[serde(rename = "console-log")]
    ConsoleLogEntry { log: BrowserLog },
    #[serde(rename = "network-request")]
    NetworkRequestEntry { request: NetworkRequest },
    #[serde(rename = "element-selected")]
    ElementSelected { element: SelectedElement },
}

/// MCP Tool definition (for SR-03 tool listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool call (for MCP protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Page info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub url: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a tool definition
pub fn tool(name: &str, description: &str, params: &[(&str, &str, &str)]) -> Tool {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for (name, typ, desc) in params {
        properties.insert(
            name.to_string(),
            json!({
                "type": typ,
                "description": desc,
            }),
        );
        required.push(name.to_string());
    }

    Tool {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": required,
        }),
    }
}

/// Create a text content block
pub fn text_content(text: impl Into<String>) -> ContentBlock {
    ContentBlock::Text { text: text.into() }
}

/// Create an image content block
pub fn image_content(data: String, mime_type: String) -> ContentBlock {
    ContentBlock::Image { data, mime_type }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let tool = tool(
            "test_tool",
            "A test tool",
            &[("param1", "string", "First parameter")],
        );

        assert_eq!(tool.name, "test_tool");
        assert!(tool.input_schema["properties"]["param1"].is_object());
    }

    #[test]
    fn test_jsonrpc_response() {
        let resp = JsonRpcResponse::ok(json!(1), json!("test"));
        assert_eq!(resp.id, json!(1));
        assert_eq!(resp.result, Some(json!("test")));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_content_block_text() {
        let content = text_content("Hello, world!");
        matches!(content, ContentBlock::Text { .. });
    }

    // Extension message tests
    #[test]
    fn test_extension_message_screenshot_data() {
        let json = r#"{"type":"screenshot-data","data":"iVBORw0KGgoAAAANSUhEUgAAAAUAAA..." ,"path":"/tmp/screenshot.png"}"#;
        let msg: ExtensionMessage = serde_json::from_str(json).unwrap();
        matches!(msg, ExtensionMessage::ScreenshotData { .. });
    }

    #[test]
    fn test_extension_message_url_response() {
        let json = r#"{"type":"current-url-response","url":"https://example.com","tab_id":123}"#;
        let msg: ExtensionMessage = serde_json::from_str(json).unwrap();
        matches!(msg, ExtensionMessage::UrlResponse { .. });
    }

    #[test]
    fn test_extension_message_console_log() {
        let json = r#"{"type":"console-log","log":{"level":"info","message":"test","timestamp":1234567890}}"#;
        let msg: ExtensionMessage = serde_json::from_str(json).unwrap();
        matches!(msg, ExtensionMessage::ConsoleLogEntry { .. });
    }

    #[test]
    fn test_selected_element() {
        let json = r#"{"tag_name":"div","id":"main","class_name":"container","attributes":{"data-id":"123"}}"#;
        let elem: SelectedElement = serde_json::from_str(json).unwrap();
        assert_eq!(elem.tag_name, "div");
        assert_eq!(elem.id, Some("main".to_string()));
    }

    #[test]
    fn test_mcp_tool() {
        let tool = McpTool {
            name: "getScreenshot".to_string(),
            description: "Capture screenshot".to_string(),
            input_schema: json!({"type": "object"}),
        };
        assert_eq!(tool.name, "getScreenshot");
    }

    #[test]
    fn test_tool_call() {
        let call = ToolCall {
            id: "call-123".to_string(),
            name: "getScreenshot".to_string(),
            arguments: json!({}),
        };
        assert_eq!(call.id, "call-123");
    }
}
