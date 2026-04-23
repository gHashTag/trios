//! # trios-mcp — MCP Client Adapter
//!
//! Rust MCP client adapter for connecting Perplexity Agents to browser-tools-server
//! via WebSocket tunnel with authentication.
//!
//! ## Example
//!
//! ```no_run
//! use trios_mcp::{ConnectionConfig, McpClient};
//!
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let config = ConnectionConfig::new("127.0.0.1", 3025)
//!     .with_auth(trios_mcp::AuthConfig::new("perplexity", "test123"));
//!
//! let mut client = McpClient::new(config);
//! client.connect().await?;
//!
//! let tools = client.list_tools().await?;
//! println!("Available tools: {:?}", tools);
//! # Ok(())
//! # }
//! ```

pub use trios_mcp_sr00::{
    JsonRpcError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse,
    McpAnnotations, McpClientCapabilities, McpClientInfo, McpContent,
    McpInitializeParams, McpResource, McpServerCapabilities, McpTool, McpToolResult,
};

pub use trios_mcp_sr01::{
    AuthConfig, ConnectionConfig, McpWebSocketClient, health_check,
};

use anyhow::{Context, Result};
use serde_json::json;
use tracing::{debug, info, warn};

/// MCP Client — main interface for browser-tools-server interaction
pub struct McpClient {
    config: ConnectionConfig,
    ws_client: McpWebSocketClient,
    initialized: bool,
}

impl McpClient {
    /// Create new MCP client with configuration
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            ws_client: McpWebSocketClient::new(config.clone()),
            config,
            initialized: false,
        }
    }

    /// Connect to the server and initialize MCP session
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to MCP server at {}:{}", self.config.host, self.config.port);

        // Check health first
        match health_check(&self.config).await {
            Ok(true) => info!("Health check passed"),
            Ok(false) => warn!("Health check failed, connecting anyway"),
            Err(e) => warn!("Health check error: {}, connecting anyway", e),
        }

        // Connect WebSocket
        self.ws_client.connect().await?;

        // Initialize MCP session
        self.initialize().await?;

        Ok(())
    }

    /// Initialize MCP session
    async fn initialize(&mut self) -> Result<()> {
        let init_params = McpInitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpClientCapabilities {
                roots: None,
                sampling: None,
            },
            client_info: Some(McpClientInfo {
                name: "trios-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }),
        };

        let request = JsonRpcRequest::new("initialize")
            .with_id(json!(1))
            .with_params(serde_json::to_value(init_params)?);

        let response = self.ws_client.send_request(&request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("Initialize failed: {}", error.message);
        }

        // Send initialized notification
        let initialized = JsonRpcRequest::new("notifications/initialized");
        let message = trios_mcp_sr00::JsonRpcMessage::Request(initialized);
        self.ws_client.send_raw(tokio_tungstenite::tungstenite::protocol::Message::Text(
            message.to_json()?
        )).await?;

        self.initialized = true;
        info!("MCP session initialized");

        Ok(())
    }

    /// List available tools from the server
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::new("tools/list")
            .with_id(json!(2));

        let response = self.ws_client.send_request(&request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("List tools failed: {}", error.message);
        }

        let result = response.result.context("No result in response")?;

        let tools: Vec<McpTool> = serde_json::from_value(
            result.get("tools").context("No tools in result")?.clone()
        )?;

        debug!("Found {} tools", tools.len());

        Ok(tools)
    }

    /// List available resources from the server
    pub async fn list_resources(&mut self) -> Result<Vec<McpResource>> {
        self.ensure_initialized()?;

        let request = JsonRpcRequest::new("resources/list")
            .with_id(json!(3));

        let response = self.ws_client.send_request(&request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("List resources failed: {}", error.message);
        }

        let result = response.result.context("No result in response")?;

        let resources: Vec<McpResource> = serde_json::from_value(
            result.get("resources").context("No resources in result")?.clone()
        )?;

        Ok(resources)
    }

    /// Call a tool with given parameters
    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult> {
        self.ensure_initialized()?;

        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let request = JsonRpcRequest::new("tools/call")
            .with_id(json!(4))
            .with_params(params);

        let response = self.ws_client.send_request(&request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("Call tool failed: {}", error.message);
        }

        let result = response.result.context("No result in response")?;
        let tool_result: McpToolResult = serde_json::from_value(result)?;

        Ok(tool_result)
    }

    /// Read a resource by URI
    pub async fn read_resource(&mut self, uri: &str) -> Result<Vec<McpContent>> {
        self.ensure_initialized()?;

        let params = json!({ "uri": uri });

        let request = JsonRpcRequest::new("resources/read")
            .with_id(json!(5))
            .with_params(params);

        let response = self.ws_client.send_request(&request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("Read resource failed: {}", error.message);
        }

        let result = response.result.context("No result in response")?;

        let contents: Vec<McpContent> = serde_json::from_value(
            result.get("contents").context("No contents in result")?.clone()
        )?;

        Ok(contents)
    }

    /// Get server capabilities
    pub async fn get_server_capabilities(&mut self) -> Result<McpServerCapabilities> {
        let request = JsonRpcRequest::new("initialize")
            .with_id(json!(1))
            .with_params(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "trios-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }));

        let response = self.ws_client.send_request(&request).await?;

        if let Some(error) = response.error {
            anyhow::bail!("Get capabilities failed: {}", error.message);
        }

        let result = response.result.context("No result in response")?;
        let caps: McpServerCapabilities = serde_json::from_value(
            result.get("capabilities").context("No capabilities in result")?.clone()
        )?;

        Ok(caps)
    }

    /// Ping the server
    pub async fn ping(&mut self) -> Result<bool> {
        let request = JsonRpcRequest::new("ping")
            .with_id(json!(6));

        match self.ws_client.send_request(&request).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Ping failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.ws_client.is_connected()
    }

    /// Check if client is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Ensure client is initialized
    fn ensure_initialized(&self) -> Result<()> {
        if !self.initialized {
            anyhow::bail!("Client not initialized. Call connect() first.");
        }
        Ok(())
    }

    /// Close the connection
    pub async fn close(self) -> Result<()> {
        self.ws_client.close().await
    }

    /// Get the underlying WebSocket client
    pub fn ws_client(&self) -> &McpWebSocketClient {
        &self.ws_client
    }

    /// Get mutable reference to the WebSocket client
    pub fn ws_client_mut(&mut self) -> &mut McpWebSocketClient {
        &mut self.ws_client
    }
}

impl From<ConnectionConfig> for McpClient {
    fn from(config: ConnectionConfig) -> Self {
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_from_config() {
        let config = ConnectionConfig::new("localhost", 3025);
        let client = McpClient::from(config);

        assert_eq!(client.config.host, "localhost");
        assert_eq!(client.config.port, 3025);
        assert!(!client.is_connected());
        assert!(!client.is_initialized());
    }
}
