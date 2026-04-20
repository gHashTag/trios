use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExtensionState {
    pub sidepanel_open: bool,
    pub current_tab: String,
    pub chat_log: Vec<serde_json::Value>,
    pub agents: Vec<serde_json::Value>,
    pub errors: Vec<String>,
}

impl Default for ExtensionState {
    fn default() -> Self {
        Self {
            sidepanel_open: false,
            current_tab: "chat".to_string(),
            chat_log: Vec::new(),
            agents: Vec::new(),
            errors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OperatorParams {
    pub token: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SendChatRequest {
    pub message: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClickRequest {
    pub selector: String,
}

lazy_static::lazy_static! {
    static ref STATE: Arc<std::sync::Mutex<ExtensionState>> =
        Arc::new(std::sync::Mutex::new(ExtensionState::default()));
}

fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

fn validate_token(token: &str, expected: &str) -> bool {
    token == expected
}

pub async fn ping(Query(params): Query<OperatorParams>) -> impl IntoResponse {
    let expected_token = std::env::var("OPERATOR_TOKEN")
        .unwrap_or_else(|_| "dev-token".to_string());

    if !validate_token(&params.token, &expected_token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid_token"})));
    }

    (StatusCode::OK, Json(serde_json::json!({
        "status": "ok",
        "server": "trios-server",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": 0
    })))
}

pub async fn get_extension_state(Query(params): Query<OperatorParams>) -> impl IntoResponse {
    let expected_token = std::env::var("OPERATOR_TOKEN")
        .unwrap_or_else(|_| "dev-token".to_string());

    if !validate_token(&params.token, &expected_token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid_token"})));
    }

    let state = STATE.lock().unwrap();
    (StatusCode::OK, Json(serde_json::json!({
        "sidepanel_open": state.sidepanel_open,
        "current_tab": state.current_tab,
        "chat_log": state.chat_log,
        "agents": state.agents,
        "errors": state.errors,
        "timestamp": chrono::Utc::now().timestamp()
    })))
}

pub async fn send_chat(
    Query(params): Query<OperatorParams>,
    Json(req): Json<SendChatRequest>,
) -> impl IntoResponse {
    let expected_token = std::env::var("OPERATOR_TOKEN")
        .unwrap_or_else(|_| "dev-token".to_string());

    if !validate_token(&params.token, &expected_token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid_token"})));
    }

    let message_id = Uuid::new_v4().to_string();
    let mut state = STATE.lock().unwrap();
    state.chat_log.push(serde_json::json!({
        "role": "operator",
        "content": req.message,
        "timestamp": chrono::Utc::now().timestamp()
    }));

    (StatusCode::OK, Json(serde_json::json!({
        "sent": true,
        "message_id": message_id
    })))
}

pub async fn click(
    Query(params): Query<OperatorParams>,
    Json(req): Json<ClickRequest>,
) -> impl IntoResponse {
    let expected_token = std::env::var("OPERATOR_TOKEN")
        .unwrap_or_else(|_| "dev-token".to_string());

    if !validate_token(&params.token, &expected_token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid_token"})));
    }

    let mut state = STATE.lock().unwrap();
    state.errors.push(format!("Click requested: {}", req.selector));

    (StatusCode::OK, Json(serde_json::json!({"clicked": req.selector})))
}

pub fn init_operator_token() -> String {
    let token = generate_token();
    std::env::set_var("OPERATOR_TOKEN", token.clone());
    tracing::info!("Operator token generated: {}", token);
    token
}
