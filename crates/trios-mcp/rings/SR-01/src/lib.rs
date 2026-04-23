//! # SR-01: Auth + WebSocket
//!
//! Basic Auth and WebSocket client for browser-tools-server connection.

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{connect_async_with_config, WebSocketStream};
use tracing::{debug, error, info, warn};
use url::Url;

use trios_mcp_sr00::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse};

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
}

impl AuthConfig {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Convert to Basic Auth header value (Base64)
    pub fn to_header_value(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        format!("Basic {}", BASE64.encode(credentials))
    }

    /// Create from Base64 Basic Auth header
    pub fn from_base64(auth_header: &str) -> Result<Self> {
        let auth_header = auth_header.strip_prefix("Basic ")
            .context("Invalid Basic Auth header format")?;

        let decoded = BASE64.decode(auth_header)
            .context("Failed to decode Base64")?;

        let credentials = String::from_utf8(decoded)
            .context("Invalid UTF-8 in credentials")?;

        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid credentials format");
        }

        Ok(Self {
            username: parts[0].to_string(),
            password: parts[1].to_string(),
        })
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::new("perplexity", "test123")
    }
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub auth: AuthConfig,
    pub use_tls: bool,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: usize,
}

impl ConnectionConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            auth: AuthConfig::default(),
            use_tls: false,
            reconnect_interval: Duration::from_secs(5),
            max_reconnect_attempts: 10,
        }
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }

    pub fn with_reconnect(mut self, interval: Duration, max_attempts: usize) -> Self {
        self.reconnect_interval = interval;
        self.max_reconnect_attempts = max_attempts;
        self
    }

    pub fn ws_url(&self) -> Result<Url> {
        let scheme = if self.use_tls { "wss" } else { "ws" };
        Url::parse(&format!("{}://{}:{}/", scheme, self.host, self.port))
            .context("Failed to construct WebSocket URL")
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self::new("127.0.0.1", 3025)
    }
}

/// WebSocket client with auto-reconnect
pub struct McpWebSocketClient {
    config: ConnectionConfig,
    stream: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
    reconnect_count: usize,
}

impl McpWebSocketClient {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            stream: None,
            reconnect_count: 0,
        }
    }

    /// Connect to the server with authentication
    pub async fn connect(&mut self) -> Result<()> {
        let url = self.config.ws_url()?;

        debug!("Connecting to WebSocket server at {}", url);

        let mut request = url.into_client_request()
            .context("Failed to create WebSocket request")?;

        // Add Basic Auth header
        request.headers_mut().insert(
            "Authorization",
            self.config.auth.to_header_value().parse()
                .context("Failed to parse Authorization header")?,
        );

        let config = tokio_tungstenite::tungstenite::client::WebSocketConfig {
            max_message_size: Some(64 * 1024 * 1024), // 64MB
            max_send_queue: None,
            accept_unmasked_frames: false,
        };

        let (stream, response) = connect_async_with_config(request, Some(config), false)
            .await
            .context("Failed to connect to WebSocket server")?;

        info!("Connected to server. Response: {:?}", response.status());

        self.stream = Some(stream);
        self.reconnect_count = 0;

        Ok(())
    }

    /// Reconnect with exponential backoff
    pub async fn reconnect(&mut self) -> Result<()> {
        if self.reconnect_count >= self.config.max_reconnect_attempts {
            anyhow::bail!("Max reconnect attempts reached");
        }

        self.reconnect_count += 1;

        let delay = self.config.reconnect_interval * self.reconnect_count as u32;
        info!("Reconnect attempt {}/{}. Waiting {:?}",
              self.reconnect_count, self.config.max_reconnect_attempts, delay);

        tokio::time::sleep(delay).await;

        self.connect().await
    }

    /// Send a JSON-RPC request
    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        self.ensure_connected().await?;

        let message = Message::Text(JsonRpcMessage::Request(request.clone()).to_json()?);

        debug!("Sending request: {}", message);

        self.send_raw(message).await?;
        self.receive_response().await
    }

    /// Send raw message
    pub async fn send_raw(&mut self, message: Message) -> Result<()> {
        let stream = self.stream.as_mut()
            .context("Not connected")?;

        stream.send(message).await
            .context("Failed to send message")?;

        Ok(())
    }

    /// Receive next response
    pub async fn receive_response(&mut self) -> Result<JsonRpcResponse> {
        let stream = self.stream.as_mut()
            .context("Not connected")?;

        let message = stream.next().await
            .context("Connection closed")?
            .context("Failed to receive message")?;

        match message {
            Message::Text(text) => {
                debug!("Received: {}", text);

                match JsonRpcMessage::from_json(&text)? {
                    JsonRpcMessage::Response(resp) => Ok(resp),
                    JsonRpcMessage::Request(_) => {
                        anyhow::bail!("Expected response, got request")
                    }
                }
            }
            Message::Close(_) => {
                warn!("Server closed connection");
                self.reconnect().await?;
                self.receive_response().await
            }
            msg => anyhow::bail!("Unexpected message type: {:?}", msg),
        }
    }

    /// Ensure connection is active, reconnect if needed
    async fn ensure_connected(&mut self) -> Result<()> {
        if self.stream.is_none() {
            self.connect().await
        } else {
            Ok(())
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Get current reconnect count
    pub fn reconnect_count(&self) -> usize {
        self.reconnect_count
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.close(None).await
                .context("Failed to close connection")?;
        }
        Ok(())
    }
}

/// Simple health check via HTTP
pub async fn health_check(config: &ConnectionConfig) -> Result<bool> {
    let url = if config.use_tls {
        format!("https://{}:{}/.identity", config.host, config.port)
    } else {
        format!("http://{}:{}/.identity", config.host, config.port)
    };

    debug!("Health check: {}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let mut request = client.get(&url);
    request = request.header("Authorization", config.auth.to_header_value());

    let response = request.send().await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        Ok(body.get("signature").and_then(|s| s.as_str())
            == Some("mcp-browser-connector-24x7"))
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config() {
        let auth = AuthConfig::new("user", "pass");
        let header = auth.to_header_value();

        assert!(header.starts_with("Basic "));

        let decoded = AuthConfig::from_base64(&header).unwrap();
        assert_eq!(decoded.username, "user");
        assert_eq!(decoded.password, "pass");
    }

    #[test]
    fn test_connection_config() {
        let config = ConnectionConfig::new("localhost", 3025);

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 3025);
        assert_eq!(config.auth.username, "perplexity");
    }

    #[test]
    fn test_ws_url() {
        let config = ConnectionConfig::new("127.0.0.1", 3025);
        let url = config.ws_url().unwrap();

        assert_eq!(url.scheme(), "ws");
        assert_eq!(url.host_str().unwrap(), "127.0.0.1");
        assert_eq!(url.port().unwrap(), 3025);
    }
}
