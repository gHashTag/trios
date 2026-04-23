//! WebSocket handler for Chrome Extension communication
//!
//! Handles bidirectional WebSocket messages:
//! - From extension: console logs, network requests, page navigation, selected element, screenshots
//! - To extension: take screenshot, server shutdown

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    tungstenite::protocol::Message,
    WebSocketStream,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::logs::{ConsoleLog, ConsoleError, NetworkRequest, SelectedElement};

/// WebSocket message types from Chrome Extension
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtensionMessage {
    /// Console log message
    #[serde(rename = "console-log")]
    ConsoleLog {
        #[serde(default)]
        level: Option<String>,
        message: Option<String>,
        url: Option<String>,
        #[serde(default)]
        timestamp: Option<String>,
    },

    /// Console error message
    #[serde(rename = "console-error")]
    ConsoleError {
        level: String,
        message: String,
        #[serde(default)]
        timestamp: Option<String>,
    },

    /// Network request
    #[serde(rename = "network-request")]
    NetworkRequest {
        url: String,
        #[serde(default)]
        method: Option<String>,
        status: Option<i64>,
        #[serde(default)]
        timestamp: Option<String>,
    },

    /// Page navigation event
    #[serde(rename = "page-navigated")]
    PageNavigated {
        url: String,
    #[serde(default)]
        timestamp: Option<String>>,
    },

    /// DOM element selection
    #[serde(rename = "selected-element")]
    SelectedElement {
        element: serde_json::Value,
    },

    /// Current URL update response
    #[serde(rename = "current-url-response")]
    CurrentUrlResponse {
        url: String,
    },

    /// Screenshot data (base64 PNG)
    #[serde(rename = "screenshot-data")]
    ScreenshotData {
        request_id: String,
        data: String,
    },

    /// Screenshot error response
    #[serde(rename = "screenshot-error")]
    ScreenshotError {
        request_id: String,
        error: String,
    },
}

/// WebSocket message types to send to Chrome Extension
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Request screenshot capture
    #[serde(rename = "take-screenshot")]
    TakeScreenshot {
        request_id: String,
    },

    /// Notify of server shutdown
    #[serde(rename = "server-shutdown")]
    ServerShutdown {},
}

/// WebSocket state
pub struct WebSocketState {
    pub logs: Arc<tokio::sync::Mutex<crate::logs::LogStore>>,
}

impl ExtensionMessage {
    /// Convert to log store operations
    pub fn process(&self, logs: &mut crate::logs::LogStore) {
        match self {
            ExtensionMessage::ConsoleLog(msg) => {
                let log = crate::logs::ConsoleLog {
                    log_type: "console-log".to_string(),
                    level: msg.level,
                    message: msg.message,
                    url: msg.url,
                    timestamp: msg.timestamp.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                };
                logs.add_console_log(log);
                debug!("Console log: {:?}", msg);
            }
            ExtensionMessage::ConsoleError(msg) => {
                let err = crate::logs::ConsoleError {
                    error_type: "console-error".to_string(),
                    level: msg.level,
                    message: msg.message,
                    timestamp: msg.timestamp.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                };
                logs.add_console_error(err);
                debug!("Console error: {:?}", msg);
            }
            ExtensionMessage::NetworkRequest(msg) => {
                let req = crate::logs::NetworkRequest {
                    request_type: "xhr".to_string(),
                    url: msg.url,
                    method: msg.method,
                    status_code: msg.status,
                    timestamp: msg.timestamp.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                };
                logs.add_network_request(req);
                debug!("Network request: {:?}", msg);
            }
            ExtensionMessage::PageNavigated(msg) => {
                logs.set_current_url(msg.url);
                info!("Page navigated to: {}", msg.url);
            }
            ExtensionMessage::SelectedElement(msg) => {
                if let Ok(el) = serde_json::from_value(msg.element) {
                    logs.set_selected_element(crate::logs::SelectedElement::Element(el));
                    debug!("Selected element: {:?}", el);
                }
            }
            ExtensionMessage::CurrentUrlResponse(msg) => {
                logs.set_current_url(msg.url);
                debug!("Current URL response: {}", msg.url);
            }
            ExtensionMessage::ScreenshotData(msg) => {
                // TODO: Save screenshot to file
                info!("Screenshot received, request_id: {}", msg.request_id);
            }
            ExtensionMessage::ScreenshotError(msg) => {
                warn!("Screenshot error: {}, {}", msg.request_id, msg.error);
            }
        }
    }
}

/// Handle incoming WebSocket message
pub fn handle_message(
    message: Message,
    ws_state: &WebSocketState,
) -> Result<()> {
    match message {
        Message::Text(text) => {
            let ext_msg: ExtensionMessage = serde_json::from_str(&text)?;
            let mut logs = ws_state.logs.lock().await;
            ext_msg.process(&mut logs);
            Ok(())
        }
        Message::Close(close_frame) => {
            info!("WebSocket closed: {:?}", close_frame);
            Ok(())
        }
        Message::Ping(ping) => {
            // Respond to ping
            Ok(())
        }
        Message::Pong(_) => {
            Ok(())
        }
        msg => {
            warn!("Unexpected WebSocket message: {:?}", msg);
            Ok(())
        }
    }
}

/// Send message to Chrome Extension via WebSocket
pub async fn send_to_extension(
    ws: &mut WebSocketStream<...>,
    message: &ServerMessage,
) -> Result<()> {
    let json = serde_json::to_string(message)?;
    let msg = Message::Text(json);

    ws.send(msg).await?;

    Ok(())
}

/// Create take-screenshot message with unique request ID
pub fn create_screenshot_request() -> ServerMessage {
    ServerMessage::TakeScreenshot {
        request_id: Uuid::new_v4().to_string(),
    }
}

/// Create server shutdown message
pub const fn create_shutdown_message() -> ServerMessage {
    ServerMessage::ServerShutdown {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_message_console_log() {
        let json = r#"{"type":"console-log","level":"info","message":"test"}"#;
        let msg: ExtensionMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ExtensionMessage::ConsoleLog(_)));
    }

    #[test]
    fn test_extension_message_network_request() {
        let json = r#"{"type":"network-request","url":"https://example.com","method":"GET","status":200}"#;
        let msg: ExtensionMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ExtensionMessage::NetworkRequest(_)));
    }

    #[test]
    fn test_server_message_take_screenshot() {
        let msg = ServerMessage::create_screenshot_request();
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("take-screenshot"));
        assert!(json.contains("requestId"));
    }

    #[test]
    fn test_server_message_shutdown() {
        let msg = ServerMessage::create_shutdown_message();
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"server-shutdown"}{}"#);
    }
}
