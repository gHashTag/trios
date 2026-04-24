mod mcp;
mod mcp_endpoints;
mod operator;
mod security;
mod sse_handler;
mod tools;
mod ws_handler;

use axum::extract::{Query, State};
use axum::response::Json;
use axum::Router;
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use ws_handler::AppState;

#[derive(Deserialize)]
struct AgentIdQuery {
    agent_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trios_server=debug,tower_http=debug")
        .init();

    // Load .env from workspace root (manual parser — handles inline comments)
    load_dot_env();

    let operator_token = operator::init_operator_token();
    info!("Operator token: {}", operator_token);

    // Load z.ai config from environment
    let zai_api = std::env::var("ZAI_API").unwrap_or_default();
    let zai_keys: Vec<String> = (1..=6)
        .filter_map(|i| std::env::var(format!("ZAI_KEY_{}", i)).ok())
        .filter(|k| !k.is_empty())
        .collect();
    if !zai_api.is_empty() {
        info!("z.ai endpoint: {}", zai_api);
        info!("z.ai keys loaded: {}", zai_keys.len());
    }

    let state = AppState::with_zai(zai_api, zai_keys);
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
        .route("/mcp/browser-commands", get(browser_poll))
        .route("/mcp/browser-result", post(browser_report))
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

async fn browser_poll(
    State(state): State<AppState>,
    Query(params): Query<AgentIdQuery>,
) -> Json<Value> {
    if params.agent_id.is_empty() {
        return Json(json!({"error": "agent_id is required"}));
    }
    let result = mcp_endpoints::browser::browser_commands(&state, &params.agent_id).await;
    Json(result)
}

/// Manual .env parser — handles inline comments (dotenvy chokes on our format)
fn load_dot_env() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let env_path = cwd.join(".env");
    if !env_path.exists() {
        info!(".env not found at {}", env_path.display());
        return;
    }
    let content = std::fs::read_to_string(&env_path).unwrap_or_default();
    let mut count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            // Strip inline comments (# not inside quotes)
            let val = v.split_once('#').map(|(v, _)| v).unwrap_or(v);
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() && !val.is_empty() {
                // Only set if not already in environment (env vars take precedence)
                if std::env::var(&key).is_err() {
                    std::env::set_var(&key, &val);
                    count += 1;
                }
            }
        }
    }
    info!(".env loaded: {} vars from {}", count, env_path.display());
}

async fn browser_report(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let result = mcp_endpoints::browser::browser_result(&state, body).await;
    Json(result)
}
