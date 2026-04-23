mod mcp;
mod mcp_endpoints;
mod operator;
mod security;
mod sse_handler;
mod tools;
mod ws_handler;

use axum::extract::State;
use axum::response::Json;
use axum::Router;
use axum::routing::{get, post};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use ws_handler::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trios_server=debug,tower_http=debug")
        .init();

    let operator_token = operator::init_operator_token();
    info!("Operator token: {}", operator_token);

    let state = AppState::new();
    let port: u16 = std::env::var("TRIOS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9005);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler::ws_handler))
        .route("/operator", get(operator::operator_ws_handler))
        .route("/sse", get(sse_handler::sse_handler))
        .route("/sse/message", post(sse_handler::sse_message))
        .route("/api/chat", post(api_chat))
        .route("/api/status", get(api_status))
        .route("/mcp/browser-commands", get(mcp_endpoints::browser::poll_commands))
        .route("/mcp/browser-result", post(mcp_endpoints::browser::report_result))
        .route("/health", get(health))
        .route("/", get(health))
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(axum::middleware::from_fn(security::auth_middleware))
                .layer(axum::middleware::from_fn(security::timeout_middleware)),
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("trios-server listening on 0.0.0.0:{}", port);
    info!("  WS:  ws://0.0.0.0:{}/ws", port);
    info!("  SSE: http://0.0.0.0:{}/sse  (Claude Desktop / Cursor)", port);
    info!("  REST: http://0.0.0.0:{}/api/chat", port);
    info!("  BrowserOS: http://0.0.0.0:{}/mcp/browser-commands", port);
    info!("  MCP tools: {} registered", tools::count());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn api_chat(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let text = serde_json::to_string(&body).unwrap_or_default();
    let response = ws_handler::handle_message(&text, &state).await;
    Json(json!({"result": response.result}))
}

async fn api_status(State(state): State<AppState>) -> Json<Value> {
    let agents = state.agents.lock().await.len();
    Json(json!({
        "status": "ok",
        "agents": agents,
        "tools": tools::count(),
    }))
}
