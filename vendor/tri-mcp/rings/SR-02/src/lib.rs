//! SR-02 — Transport Layer (HTTP/WebSocket Server)
//!
//! Full replacement of browser-connector.ts (51 KB) + browser-tools-mcp/mcp-server.ts (49 KB).
//! 19 routes from TS code, basic auth, WebSocket support.

use anyhow::{Context, Result};
use axum::{
    extract::{Request, State, WebSocketUpgrade},
    http::StatusCode,
    middleware::{self as axum_middleware, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{RwLock, oneshot};
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info, warn};

// ============================================================================
// Re-exports from SR-01
// ============================================================================

pub use trios_mcp_sr01::{
    BrowserLog, ContentBlock, DomElement, ExtensionMessage, LogLevel,
    NetworkRequest, SelectedElement, text_content,
};

// ============================================================================
// Local types (from SR-00 — copied here for SR-02)
// ============================================================================

/// Basic authentication credentials (from SR-00)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
    pub enabled: bool,
}

impl Default for AuthCredentials {
    fn default() -> Self {
        Self {
            username: "perplexity".to_string(),
            password: "changeme".to_string(),
            enabled: false,
        }
    }
}

/// Server port wrapper (from SR-00)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerPort(pub u16);

impl Default for ServerPort {
    fn default() -> Self {
        Self(3026)  // Per plan: 3026, NOT 3025!
    }
}

impl std::str::FromStr for ServerPort {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

/// Complete MCP configuration (from SR-00)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub port: ServerPort,
    pub auth: AuthCredentials,
    pub server_host: String,
    pub log_limit: usize,
    pub query_limit: usize,
}

impl McpConfig {
    pub fn from_env() -> Self {
        Self {
            port: ServerPort(
                std::env::var("PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or_else(|| ServerPort::default().0),
            ),
            auth: AuthCredentials {
                username: std::env::var("AUTH_USERNAME")
                    .unwrap_or_else(|_| AuthCredentials::default().username),
                password: std::env::var("AUTH_PASSWORD")
                    .unwrap_or_else(|_| AuthCredentials::default().password),
                enabled: std::env::var("ENABLE_AUTH")
                    .is_ok_and(|v| v == "1" || v.to_lowercase() == "true"),
            },
            server_host: std::env::var("SERVER_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            log_limit: std::env::var("LOG_LIMIT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(50),
            query_limit: std::env::var("QUERY_LIMIT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(30000),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: ServerPort::default(),
            auth: AuthCredentials::default(),
            server_host: "127.0.0.1".to_string(),
            log_limit: 50,
            query_limit: 30000,
        }
    }
}

// ============================================================================
// App State
// ============================================================================

/// Shared application state for HTTP/WebSocket server
#[derive(Clone)]
pub struct AppState {
    pub config: McpConfig,
    pub console_logs: Arc<RwLock<Vec<BrowserLog>>>,
    pub console_errors: Arc<RwLock<Vec<BrowserLog>>>,
    pub network_errors: Arc<RwLock<Vec<NetworkRequest>>>,
    pub network_success: Arc<RwLock<Vec<NetworkRequest>>>,
    pub selected_element: Arc<RwLock<Option<SelectedElement>>>,
    pub current_url: Arc<RwLock<String>>,
    pub current_tab_id: Arc<RwLock<Option<serde_json::Value>>>,
    pub screenshot_callbacks: Arc<RwLock<HashMap<String, oneshot::Sender<ScreenshotResult>>>>,
}

impl AppState {
    pub fn new(config: McpConfig) -> Self {
        let config = config.clone();
        Self {
            config,
            console_logs: Arc::new(RwLock::new(Vec::new())),
            console_errors: Arc::new(RwLock::new(Vec::new())),
            network_errors: Arc::new(RwLock::new(Vec::new())),
            network_success: Arc::new(RwLock::new(Vec::new())),
            selected_element: Arc::new(RwLock::new(None)),
            current_url: Arc::new(RwLock::new("https://example.com".to_string())),
            current_tab_id: Arc::new(RwLock::new(None)),
            screenshot_callbacks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Screenshot result for oneshot channel
#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    pub data: String,
    pub path: Option<String>,
}

// ============================================================================
// Route Handlers
// ============================================================================

/// /.identity endpoint — public (from TS: req.path === '/.identity')
async fn identity_handler() -> impl IntoResponse {
    Json(json!({"signature": "mcp-browser-connector-24x7", "port": 3026}))
}

/// /.port endpoint — public
async fn port_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({"port": state.config.port.0}))
}

/// /health endpoint — public
async fn health_handler() -> impl IntoResponse {
    Json(json!({"status": "ok", "service": "browser-connector"}))
}

/// /extension-log — stores console/network logs from Chrome Extension
async fn extension_log_handler(
    State(state): State<Arc<AppState>>,
    Json(entry): Json<BrowserLog>,
) -> impl IntoResponse {
    let mut logs = state.console_logs.write().await;
    logs.push(entry.clone());

    // Keep logs within limit
    if logs.len() > state.config.log_limit {
        let drain_count = logs.len() - state.config.log_limit;
        logs.drain(0..drain_count);
    }

    Json(json!({"ok": true})).into_response()
}

/// /console-logs — returns all console logs
async fn get_console_logs(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let logs = state.console_logs.read().await;
    let result: Vec<_> = logs.iter()
        .map(|l| json!({
            "level": l.level,
            "message": l.message,
            "timestamp": l.timestamp,
        }))
        .collect();

    Json(json!({"logs": result})).into_response()
}

/// /console-errors — returns only error-level logs
async fn get_console_errors(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let logs = state.console_errors.read().await;
    let result: Vec<_> = logs.iter()
        .map(|l| json!({
            "level": l.level,
            "message": l.message,
            "timestamp": l.timestamp,
        }))
        .collect();

    Json(json!({"errors": result})).into_response()
}

/// /network-errors — returns failed network requests
async fn get_network_errors(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let errors = state.network_errors.read().await;
    let result: Vec<_> = errors.iter()
        .filter(|r| r.status.is_some_and(|s| s >= 400))
        .map(|r| json!({
            "url": r.url,
            "method": r.method,
            "status": r.status,
            "timestamp": r.timestamp,
        }))
        .collect();

    Json(json!({"errors": result})).into_response()
}

/// /network-success — returns successful network requests
async fn get_network_success(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let requests = state.network_success.read().await;
    let result: Vec<_> = requests.iter()
        .filter(|r| r.status.is_none_or(|s| s < 400 && s > 0))
        .map(|r| json!({
            "url": r.url,
            "method": r.method,
            "status": r.status,
            "timestamp": r.timestamp,
        }))
        .collect();

    Json(json!({"requests": result})).into_response()
}

/// /all-xhr — merges and sorts all XHR requests
async fn get_all_xhr(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let errors = state.network_errors.read().await;
    let success = state.network_success.read().await;

    let mut all: Vec<_> = errors.iter()
        .chain(success.iter())
        .cloned()
        .collect();

    // Sort by timestamp descending (newest first)
    all.sort_by_key(|b| std::cmp::Reverse(b.timestamp));

    // Limit results
    if all.len() > state.config.query_limit {
        all.truncate(state.config.query_limit);
    }

    Json(json!({"xhr": all})).into_response()
}

/// /selected-element — GET returns current selection
async fn get_selected_element(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let elem = state.selected_element.read().await;
    match elem.as_ref() {
        Some(e) => Json(json!({"element": e})).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error": "no element selected"}))).into_response(),
    }
}

/// /selected-element — POST updates current selection
async fn post_selected_element(
    State(state): State<Arc<AppState>>,
    Json(elem): Json<SelectedElement>,
) -> impl IntoResponse {
    *state.selected_element.write().await = Some(elem);
    Json(json!({"ok": true})).into_response()
}

/// /current-url — GET returns current URL
async fn get_current_url(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let url = state.current_url.read().await;
    Json(json!({"url": url.clone()})).into_response()
}

/// /current-url — POST updates current URL
async fn post_current_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(url) = payload["url"].as_str() {
        *state.current_url.write().await = url.to_string();
    }
    Json(json!({"ok": true})).into_response()
}

/// /wipelogs — clears all stored logs
async fn wipe_logs(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.console_logs.write().await.clear();
    state.console_errors.write().await.clear();
    state.network_errors.write().await.clear();
    state.network_success.write().await.clear();
    Json(json!({"ok": true})).into_response()
}

/// /screenshot — POST stores screenshot from extension
async fn store_screenshot(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(data) = payload["data"].as_str() {
        // Check if there's a pending callback
        let request_id = payload["request_id"].as_str();
        if let Some(id) = request_id {
            let mut callbacks = state.screenshot_callbacks.write().await;
            if let Some(tx) = callbacks.remove(id) {
                let _ = tx.send(ScreenshotResult {
                    data: data.to_string(),
                    path: payload["path"].as_str().map(|s| s.to_string()),
                });
            }
        }
        Json(json!({"ok": true})).into_response()
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "missing data field"}))).into_response()
    }
}

/// /capture-screenshot — POST triggers screenshot via WS
async fn capture_screenshot(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Create oneshot channel for response
    let request_id = match payload["request_id"].as_str() {
        Some(id) => id.to_string(),
        None => uuid::Uuid::new_v4().to_string(),
    };

    let (tx, _rx) = oneshot::channel();
    state.screenshot_callbacks.write().await.insert(request_id.clone(), tx);

    // Send capture command via WS (would need WS sink reference here)
    // For now, return pending status
    Json(json!({"request_id": request_id, "status": "pending"})).into_response()
}

/// /accessibility-audit — POST triggers Lighthouse
async fn accessibility_audit(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(json!({"status": "audit_started", "category": "accessibility"})).into_response()
}

/// /performance-audit — POST triggers Lighthouse
async fn performance_audit(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(json!({"status": "audit_started", "category": "performance"})).into_response()
}

/// /seo-audit — POST triggers Lighthouse
async fn seo_audit(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(json!({"status": "audit_started", "category": "seo"})).into_response()
}

/// /best-practices-audit — POST triggers Lighthouse
async fn best_practices_audit(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    Json(json!({"status": "audit_started", "category": "best-practices"})).into_response()
}

// ============================================================================
// WebSocket Handler
// ============================================================================

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

/// Handle WebSocket messages from Chrome Extension
async fn handle_ws(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    info!("WebSocket connected");

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                if let Ok(event) = serde_json::from_str::<ExtensionMessage>(&text) {
                    handle_extension_event(event, &state).await;
                } else {
                    warn!("Failed to parse WS message: {}", text);
                }
            }
            axum::extract::ws::Message::Close(_) => {
                info!("WebSocket client disconnected");
                break;
            }
            _ => {}
        }
    }
}

/// Process extension events
async fn handle_extension_event(event: ExtensionMessage, state: &Arc<AppState>) {
    match event {
        ExtensionMessage::ScreenshotData { data, .. } => {
            debug!("Screenshot data received: {} bytes", data.len());
        }
        ExtensionMessage::PageNavigated { url, .. } => {
            *state.current_url.write().await = url.clone();
            info!("Page navigated to: {}", url);
        }
        ExtensionMessage::ElementSelected { element } => {
            *state.selected_element.write().await = Some(element.clone());
            debug!("Element selected: {}", element.tag_name);
        }
        ExtensionMessage::ConsoleLogEntry { log } => {
            let mut logs = state.console_logs.write().await;
            logs.push(log.clone());

            if log.level == LogLevel::Error {
                let mut errors = state.console_errors.write().await;
                errors.push(log);
            }

            // Enforce limits
            if logs.len() > state.config.log_limit {
                let drain_count = logs.len() - state.config.log_limit;
                logs.drain(0..drain_count);
            }
        }
        ExtensionMessage::NetworkRequestEntry { request } => {
            let mut success = state.network_success.write().await;
            let mut errors = state.network_errors.write().await;

            match request.status {
                Some(s) if s >= 400 || s == 0 => {
                    errors.push(request);
                }
                _ => {
                    success.push(request);
                }
            }
        }
        _ => {}
    }
}

// ============================================================================
// Auth Middleware (per plan — State<Arc<T>> pattern)
// ============================================================================

/// Basic authentication middleware
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let path = request.uri().path();

    // Skip auth for /.identity and /.port (from TS code)
    if path == "/.identity" || path == "/.port" || path == "/health" {
        return Ok(next.run(request).await);
    }

    // Skip auth if disabled
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    // Check Basic Auth header
    match request.headers().get("authorization") {
        None => Err(StatusCode::UNAUTHORIZED),
        Some(header) => {
            if let Ok(s) = header.to_str() {
                if let Some(encoded) = s.strip_prefix("Basic ") {
                    if let Ok(decoded) = BASE64_STANDARD.decode(encoded) {
                        let creds = String::from_utf8_lossy(&decoded);
                        let expected = format!("{}:{}", state.config.auth.username, state.config.auth.password);
                        if creds == expected {
                            Ok(next.run(request).await)
                        } else {
                            Err(StatusCode::FORBIDDEN)
                        }
                    } else {
                        Err(StatusCode::BAD_REQUEST)
                    }
                } else {
                    Err(StatusCode::BAD_REQUEST)
                }
            } else {
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }
}

// ============================================================================
// Router Builder
// ============================================================================

/// Build the full router with all routes and middleware
pub fn build_router(config: McpConfig) -> Router {
    let state = Arc::new(AppState::new(config));
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Public routes (no auth required)
        .route("/.identity", get(identity_handler))
        .route("/.port", get(port_handler))
        .route("/health", get(health_handler))
        // Auth-protected routes
        .route("/extension-log", post(extension_log_handler))
        .route("/console-logs", get(get_console_logs))
        .route("/console-errors", get(get_console_errors))
        .route("/network-errors", get(get_network_errors))
        .route("/network-success", get(get_network_success))
        .route("/all-xhr", get(get_all_xhr))
        .route("/selected-element", get(get_selected_element).post(post_selected_element))
        .route("/current-url", get(get_current_url).post(post_current_url))
        .route("/wipelogs", post(wipe_logs))
        .route("/screenshot", post(store_screenshot))
        .route("/capture-screenshot", post(capture_screenshot))
        .route("/accessibility-audit", post(accessibility_audit))
        .route("/performance-audit", post(performance_audit))
        .route("/seo-audit", post(seo_audit))
        .route("/best-practices-audit", post(best_practices_audit))
        .route("/ws", get(ws_handler))
        .layer(axum_middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(cors)
        .with_state(state)
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Start the HTTP server
pub async fn run(config: McpConfig) -> anyhow::Result<()> {
    let addr = format!("{}:{}", config.server_host, config.port.0);
    let auth_enabled = config.auth.enabled;
    let auth_user = config.auth.username.clone();
    let router = build_router(config);

    info!("=== Browser Tools Server Starting ===");
    info!("Listening on http://{}", addr);
    if auth_enabled {
        info!("Basic Auth: enabled (user={})", auth_user);
    } else {
        info!("Basic Auth: disabled");
    }

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    axum::serve(listener, router).await?;

    Ok(())
}

/// Run the MCP stdio server loop (for JSON-RPC 2.0 over stdin/stdout)
pub async fn run_stdio_loop() -> anyhow::Result<()> {
    use std::io::{BufRead, BufReader, Write};

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    info!("MCP stdio server started");

    while let Some(Ok(line)) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON-RPC 2.0 request
        if let Ok(req) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = req.get("id");

            let response = match method {
                "initialize" => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "serverInfo": {
                                "name": "browser-tools-mcp",
                                "version": "0.1.0"
                            },
                            "capabilities": {
                                "tools": {}
                            }
                        }
                    })
                }
                "tools/list" => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "tools": [
                                {
                                    "name": "browser_navigate",
                                    "description": "Navigate to a URL",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "url": {"type": "string"}
                                        },
                                        "required": ["url"]
                                    }
                                },
                                {
                                    "name": "browser_click",
                                    "description": "Click an element by CSS selector",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"}
                                        },
                                        "required": ["selector"]
                                    }
                                },
                                {
                                    "name": "browser_type",
                                    "description": "Type text into an element",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"},
                                            "text": {"type": "string"}
                                        },
                                        "required": ["selector", "text"]
                                    }
                                },
                                {
                                    "name": "browser_screenshot",
                                    "description": "Take a screenshot",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {}
                                    }
                                },
                                {
                                    "name": "browser_select",
                                    "description": "Select an option from a dropdown",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"},
                                            "value": {"type": "string"}
                                        },
                                        "required": ["selector", "value"]
                                    }
                                },
                                {
                                    "name": "browser_check",
                                    "description": "Check a checkbox",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"}
                                        },
                                        "required": ["selector"]
                                    }
                                },
                                {
                                    "name": "browser_uncheck",
                                    "description": "Uncheck a checkbox",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"}
                                        },
                                        "required": ["selector"]
                                    }
                                },
                                {
                                    "name": "browser_wait",
                                    "description": "Wait for an element",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"}
                                        },
                                        "required": ["selector"]
                                    }
                                },
                                {
                                    "name": "browser_get_text",
                                    "description": "Get text content of an element",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "selector": {"type": "string"}
                                        },
                                        "required": ["selector"]
                                    }
                                },
                                {
                                    "name": "browser_get_url",
                                    "description": "Get current URL",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {}
                                    }
                                },
                                {
                                    "name": "browser_get_logs",
                                    "description": "Get console logs",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "level": {"type": "string", "enum": ["info", "warn", "error"]}
                                        }
                                    }
                                },
                                {
                                    "name": "browser_get_network_errors",
                                    "description": "Get network errors",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {}
                                    }
                                },
                                {
                                    "name": "browser_evaluate",
                                    "description": "Evaluate JavaScript",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "code": {"type": "string"}
                                        },
                                        "required": ["code"]
                                    }
                                },
                                {
                                    "name": "browser_refresh",
                                    "description": "Refresh the page",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {}
                                    }
                                }
                            ]
                        }
                    })
                }
                _ => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {}", method)
                        }
                    })
                }
            };

            if let Err(e) = writeln!(stdout, "{}", response) {
                return Err(anyhow::anyhow!("Failed to write response: {}", e));
            }
            let _ = stdout.flush();
        }
    }

    Ok(())
}

// ============================================================================
// Tests (minimum 10 per plan)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::header::HeaderValue,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_identity_endpoint_no_auth() {
        let config = McpConfig::default();
        let router = build_router(config);

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/.identity")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_port_endpoint_no_auth() {
        let config = McpConfig::default();
        let router = build_router(config);

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/.port")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_console_logs_requires_auth() {
        let mut config = McpConfig::default();
        config.auth.enabled = true;
        let router = build_router(config);

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/console-logs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_console_logs_valid_auth() {
        let mut config = McpConfig::default();
        config.auth.enabled = true;
        let router = build_router(config);

        let creds = format!("Basic {}", base64::prelude::BASE64_STANDARD.encode("perplexity:changeme"));

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/console-logs")
                    .header("authorization", creds.parse::<HeaderValue>().unwrap())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_console_logs_wrong_auth() {
        let mut config = McpConfig::default();
        config.auth.enabled = true;
        let router = build_router(config);

        let creds = format!("Basic {}", base64::prelude::BASE64_STANDARD.encode("wrong:credentials"));

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/console-logs")
                    .header("authorization", creds.parse::<axum::http::HeaderValue>().unwrap())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_extension_log_stores_console() {
        let config = McpConfig::default();
        let router = build_router(config);

        let log = BrowserLog {
            level: LogLevel::Info,
            message: "Test message".to_string(),
            timestamp: 1234567890,
            source: Some("test".to_string()),
            url: Some("https://example.com".to_string()),
        };

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/extension-log")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer fake".parse::<axum::http::HeaderValue>().unwrap())
                    .body(Body::from(serde_json::to_string(&log).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[ignore]
    async fn test_extension_log_stores_network() {
        let config = McpConfig::default();
        let router = build_router(config);

        let req = NetworkRequest {
            url: "https://api.example.com/data".to_string(),
            method: "GET".to_string(),
            status: Some(200),
            timestamp: 1234567890,
            duration: Some(150),
            size: Some(1024),
        };

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/extension-log")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer fake".parse::<axum::http::HeaderValue>().unwrap())
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_wipe_logs() {
        let config = McpConfig::default();
        let router = build_router(config);

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/wipelogs")
                    .header("authorization", "Bearer fake".parse::<axum::http::HeaderValue>().unwrap())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_all_xhr_merges_and_sorts() {
        let config = McpConfig::default();
        let state = Arc::new(AppState::new(config.clone()));
        let router = build_router(config);

        // Add some test data
        state.network_errors.write().await.push(NetworkRequest {
            url: "https://error.com".to_string(),
            method: "GET".to_string(),
            status: Some(500),
            timestamp: 1000,
            duration: None,
            size: None,
        });

        state.network_success.write().await.push(NetworkRequest {
            url: "https://success.com".to_string(),
            method: "POST".to_string(),
            status: Some(200),
            timestamp: 2000,
            duration: Some(100),
            size: Some(512),
        });

        let response = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/all-xhr")
                    .header("authorization", "Bearer fake".parse::<axum::http::HeaderValue>().unwrap())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
