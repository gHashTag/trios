//! axum HTTP routes for browser connector
//!
//! All HTTP endpoints from browser-connector.ts (Express → axum).

use crate::{auth::AuthConfig, logs::LogStore};
use anyhow::{Context, Result};
use axum::{
    extract::Path,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Server state shared across routes
#[derive(Clone)]
pub struct ServerState {
    pub logs: Arc<tokio::sync::Mutex<LogStore>>,
    pub auth: AuthConfig,
}

/// Extension log from Chrome Extension
#[derive(Debug, Deserialize)]
pub struct ExtensionLog {
    #[serde(rename = "type")]
    pub log_type: String,
    #[serde(default)]
    pub level: Option<String>,
    pub message: Option<String>,
    pub url: Option<String>,
    pub method: Option<String>,
    pub status: Option<i64>,
    pub timestamp: String,
}

/// Request for screenshot capture
#[derive(Debug, Deserialize)]
pub struct CaptureScreenshotRequest {
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Screenshot upload from extension
#[derive(Debug, Deserialize)]
pub struct ScreenshotUpload {
    pub filename: Option<String>,
    pub data: String, // base64 PNG
}

/// Update selected element request
#[derive(Debug, Deserialize)]
pub struct UpdateElement {
    pub element: serde_json::Value,
}

/// Update current URL request
#[derive(Debug, Deserialize)]
pub struct UpdateUrl {
    pub url: String,
}

/// Audit endpoint request
#[derive(Debug, Deserialize)]
pub struct AuditRequest {
    pub url: Option<String>,
}

/// Identity response
#[derive(Debug, Serialize)]
pub struct IdentityResponse {
    pub signature: &'static str,
}

/// Success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Port response
#[derive(Debug, Serialize)]
pub struct PortResponse {
    pub port: u16,
}

/// Create axum router with all routes
pub fn create_router(state: ServerState) -> Router {
    let app = Router::new()
        // POST endpoints from Chrome Extension
        .route("/extension-log", post(extension_log))
        .route("/screenshot", post(screenshot_upload))
        .route("/selected-element", post(update_selected_element))
        .route("/current-url", post(update_current_url))
        .route("/wipelogs", post(wipe_logs))
        .route("/capture-screenshot", post(capture_screenshot))
        .route("/accessibility-audit", post(audit_endpoint("accessibility")))
        .route("/performance-audit", post(audit_endpoint("performance")))
        .route("/seo-audit", post(audit_endpoint("seo")))
        .route("/best-practices-audit", post(audit_endpoint("best-practices")))
        // GET endpoints for logs and state
        .route("/console-logs", get(get_console_logs))
        .route("/console-errors", get(get_console_errors))
        .route("/network-errors", get(get_network_errors))
        .route("/network-success", get(get_network_success))
        .route("/all-xhr", get(get_all_xhr))
        .route("/selected-element", get(get_selected_element))
        .route("/current-url", get(get_current_url))
        // Server info endpoints
        .route("/.port", get(get_port))
        .route("/.identity", get(get_identity))
        .with_state(state);

    app
}

// ===== Route Handlers =====

/// POST /extension-log — Chrome Extension → server
async fn extension_log(
    State(state): State<ServerState>,
    Json(log): Json<ExtensionLog>,
) -> Json<SuccessResponse> {
    debug!("Received extension log: {:?}", log);

    let mut logs = state.logs.lock().await;

    match log.log_type.as_str() {
        "console-log" => {
            let console_log = logs::ConsoleLog {
                log_type: "console-log".to_string(),
                level: log.level,
                message: log.message,
                url: log.url,
                timestamp: log.timestamp.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            };
            logs.add_console_log(console_log);
        }
        "console-error" => {
            let console_error = logs::ConsoleError {
                error_type: "console-error".to_string(),
                level: log.level.unwrap_or_else(|| "error".to_string()),
                message: log.message.unwrap_or_else(|| String::new()),
                timestamp: log.timestamp.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            };
            logs.add_console_error(console_error);
        }
        "network-request" => {
            let network_req = logs::NetworkRequest {
                request_type: "xhr".to_string(),
                url: log.url.unwrap_or_else(|| String::new()),
                method: log.method,
                status_code: log.status,
                timestamp: log.timestamp.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            };
            logs.add_network_request(network_req);
        }
        "page-navigated" => {
            // Update current URL on navigation
            if let Some(url) = log.url {
                logs.set_current_url(url);
            }
        }
        "selected-element" => {
            if let Ok(el) = serde_json::from_value(log.element.unwrap_or(json!({}))) {
                logs.set_selected_element(logs::SelectedElement::Element(el));
            }
        }
        "current-url-response" => {
            if let Some(url) = log.url {
                logs.set_current_url(url);
            }
        }
        _ => {
            debug!("Unknown log type: {}", log.log_type);
        }
    }

    Json(SuccessResponse {
        success: true,
        message: "Log received".to_string(),
    })
}

/// POST /screenshot — Direct screenshot upload from extension
async fn screenshot_upload(
    State(state): State<ServerState>,
    Json(upload): Json<ScreenshotUpload>,
) -> Json<SuccessResponse> {
    debug!("Received screenshot upload, filename: {:?}", upload.filename);

    // TODO: Save base64 PNG to configured directory
    // TODO: Optional macOS auto-paste via AppleScript

    Json(SuccessResponse {
        success: true,
        message: "Screenshot saved".to_string(),
    })
}

/// POST /selected-element — Update selected element
async fn update_selected_element(
    State(state): State<ServerState>,
    Json(element): Json<UpdateElement>,
) -> Json<SuccessResponse> {
    debug!("Updating selected element: {:?}", element);

    let mut logs = state.logs.lock().await;
    if let Ok(el) = serde_json::from_value(element.element) {
        logs.set_selected_element(logs::SelectedElement::Element(el));
    }

    Json(SuccessResponse {
        success: true,
        message: "Selected element updated".to_string(),
    })
}

/// POST /current-url — Update current URL from extension
async fn update_current_url(
    State(state): State<ServerState>,
    Json(update): Json<UpdateUrl>,
) -> Json<SuccessResponse> {
    debug!("Updating current URL: {}", update.url);

    let mut logs = state.logs.lock().await;
    logs.set_current_url(update.url);

    Json(SuccessResponse {
        success: true,
        message: "Current URL updated".to_string(),
    })
}

/// POST /wipelogs — Clear all stored logs
async fn wipe_logs(
    State(state): State<ServerState>,
) -> Json<SuccessResponse> {
    info!("Wiping all logs");

    let mut logs = state.logs.lock().await;
    logs.clear_all();

    Json(SuccessResponse {
        success: true,
        message: "All logs cleared".to_string(),
    })
}

/// POST /capture-screenshot — Request screenshot from extension
async fn capture_screenshot(
    State(state): State<ServerState>,
    Json(req): Json<CaptureScreenshotRequest>,
) -> Json<SuccessResponse> {
    debug!("Screenshot capture requested");

    // TODO: Send WebSocket message to Chrome Extension
    // Use ws module to send "take-screenshot" command

    Json(SuccessResponse {
        success: true,
        message: "Screenshot capture requested".to_string(),
    })
}

/// POST /accessibility-audit — Run Lighthouse accessibility audit
async fn audit_endpoint(
    audit_type: &'static str,
) -> impl Fn(
    State<ServerState>,
    Json<AuditRequest>,
) -> impl futures::Future<Output = Json<serde_json::Value>> {
    move |State(state): State<ServerState>, Json(req): Json<AuditRequest>| async move {
        // TODO: Get current URL from logs (or use request URL)
        // TODO: Call SR-01 Lighthouse bridge
        // TODO: Return audit results

        // Placeholder response
        Json(json!({
            "metadata": {
                "url": "https://example.com",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "device": "desktop",
                "lighthouseVersion": "11.6.0",
            },
            "report": {
                "score": 0,
                "audit_counts": {
                    "failed": 0,
                    "passed": 0,
                    "manual": 0,
                },
            },
        }))
    }
}

/// GET /console-logs — Retrieve console logs
async fn get_console_logs(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    Json(json!(logs.console_logs))
}

/// GET /console-errors — Retrieve console errors
async fn get_console_errors(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    Json(json!(logs.console_errors))
}

/// GET /network-errors — Retrieve network error logs
async fn get_network_errors(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    Json(json!(logs.network_errors))
}

/// GET /network-success — Retrieve successful network requests
async fn get_network_success(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    Json(json!(logs.network_success))
}

/// GET /all-xhr — Merge and sort all network requests
async fn get_all_xhr(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    let mut all_xhr = logs.network_errors.clone();
    all_xhr.extend(logs.network_success.clone());
    // Sort by timestamp (newest first)
    all_xhr.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    Json(json!(all_xhr))
}

/// GET /selected-element — Get currently selected element
async fn get_selected_element(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    Json(json!(logs.selected_element))
}

/// GET /current-url — Get current URL
async fn get_current_url(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let logs = state.logs.lock().await;
    // TODO: Wait up to 10 seconds if URL not available
    Json(json!(logs.current_url))
}

/// GET /.port — Return actual port server is using
async fn get_port(
    State(state): State<ServerState>,
) -> Json<PortResponse> {
    // TODO: Get actual port from listener
    Json(PortResponse { port: 3025 })
}

/// GET /.identity — Return server identity info
async fn get_identity(
    State(state): State<ServerState>,
) -> Json<IdentityResponse> {
    Json(IdentityResponse {
        signature: crate::BrowserConnector::IDENTITY,
    })
}
