use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use base64::prelude::*;
use std::{collections::VecDeque, sync::{Arc, Mutex}};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

#[derive(Clone)]
struct AppState {
    logs: Arc<Mutex<VecDeque<serde_json::Value>>>,
    screenshots: Arc<Mutex<Vec<String>>>,
    auth_username: String,
    auth_password: String,
}

fn check_auth(headers: &HeaderMap, username: &str, password: &str) -> bool {
    let Some(val) = headers.get("authorization") else { return false };
    let Ok(s) = val.to_str() else { return false };
    if let Some(encoded) = s.strip_prefix("Basic ") {
        if let Ok(decoded) = BASE64_STANDARD.decode(encoded) {
            let creds = String::from_utf8_lossy(&decoded);
            let expected = format!("{}:{}", username, password);
            return creds == expected;
        }
    }
    false
}

async fn health() -> impl IntoResponse {
    Json(json!({"status": "ok", "service": "browser-connector"}))
}

async fn identity() -> impl IntoResponse {
    Json(json!({"signature": "mcp-browser-connector-24x7", "port": 3026}))
}

async fn get_logs(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &state.auth_username, &state.auth_password) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error":"unauthorized"}))).into_response();
    }
    let logs = state.logs.lock().unwrap();
    Json(json!({"logs": logs.iter().cloned().collect::<Vec<_>>()})).into_response()
}

async fn post_log(State(state): State<AppState>, headers: HeaderMap, Json(entry): Json<serde_json::Value>) -> impl IntoResponse {
    if !check_auth(&headers, &state.auth_username, &state.auth_password) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error":"unauthorized"}))).into_response();
    }
    let mut logs = state.logs.lock().unwrap();
    if logs.len() >= 1000 { logs.pop_front(); }
    logs.push_back(entry);
    Json(json!({"ok": true})).into_response()
}

async fn delete_logs(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &state.auth_username, &state.auth_password) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error":"unauthorized"}))).into_response();
    }
    state.logs.lock().unwrap().clear();
    Json(json!({"ok": true})).into_response()
}

async fn post_screenshot(State(state): State<AppState>, headers: HeaderMap, Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    if !check_auth(&headers, &state.auth_username, &state.auth_password) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error":"unauthorized"}))).into_response();
    }
    let data = payload["data"].as_str().unwrap_or("").to_string();
    let mut shots = state.screenshots.lock().unwrap();
    shots.push(data);
    if shots.len() > 10 { shots.remove(0); }
    Json(json!({"ok": true})).into_response()
}

async fn get_screenshot(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !check_auth(&headers, &state.auth_username, &state.auth_password) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error":"unauthorized"}))).into_response();
    }
    let shots = state.screenshots.lock().unwrap();
    match shots.last() {
        Some(d) => Json(json!({"screenshot": d})).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({"error":"no screenshot"}))).into_response(),
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, _state: AppState) {
    info!("WS connected");
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(t) = msg {
            info!("WS: {}", t);
        }
    }
}

pub fn build_router(username: String, password: String) -> Router {
    let state = AppState {
        logs: Arc::new(Mutex::new(VecDeque::new())),
        screenshots: Arc::new(Mutex::new(Vec::new())),
        auth_username: username,
        auth_password: password,
    };
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    Router::new()
        .route("/.identity", get(identity))
        .route("/health", get(health))
        .route("/logs", get(get_logs).post(post_log).delete(delete_logs))
        .route("/screenshot/latest", get(get_screenshot))
        .route("/screenshot", post(post_screenshot))
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())).init();
    let port = std::env::var("PORT").unwrap_or_else(|_| "3026".into());
    let username = std::env::var("AUTH_USERNAME").unwrap_or_else(|_| "perplexity".into());
    let password = std::env::var("AUTH_PASSWORD").unwrap_or_else(|_| "changeme".into());
    let app = build_router(username, password);
    let addr = format!("0.0.0.0:{}", port);
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
