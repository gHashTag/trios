mod mcp;
mod mcp_endpoints;
mod metrics;
mod operator;
mod rainbow_routes;
mod rest_a2a;
mod rest_agent;
mod rest_browseros;
mod rest_chat;
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
    // Load .env before tracing (so env vars are available for AppState)
    let env_path = std::path::Path::new(".env");
    if env_path.exists() {
        let _ = dotenv::from_path(env_path);
    }

    tracing_subscriber::fmt()
        .with_env_filter("trios_server=debug,tower_http=debug")
        .init();

    let operator_token = operator::init_operator_token();
    info!("Operator token: {}", operator_token);

    // Open the durable A2A store when TRIOS_A2A_DB is set, hydrating the
    // registry from it; otherwise memory-only (TS parity).
    let state = AppState::new_with_persistence().await;
    // Heartbeat watchdog: prune agents that stopped sending /a2a/heartbeat.
    rest_a2a::spawn_prune_loop(state.clone());
    // Port resolution (TS-retirement item 3 — client switchover without a
    // manual reconfig). The single Rust entry point honours, in order:
    //   1. TRIOS_PORT      — explicit override for the consolidated server
    //   2. TRIOS_MCP_PORT  — the port existing Swift/mcp-bridge clients already
    //                        inject via Info.plist (9105 in prod), so pointing
    //                        them at trios-server needs no client code change
    //   3. 9005            — consolidated default
    let port: u16 = std::env::var("TRIOS_PORT")
        .or_else(|_| std::env::var("TRIOS_MCP_PORT"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9005);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // WebSocket (agents, internal tools)
        .route("/ws", get(ws_handler::ws_handler))
        .route("/operator", get(operator::operator_ws_handler))
        // SSE transport (Claude Desktop, Cursor, VSCode)
        .route("/sse", get(sse_handler::sse_handler))
        .route("/sse/message", post(sse_handler::sse_message))
        // HTTP REST
        .route("/api/chat", post(api_chat))
        .route("/api/status", get(api_status))
        // Consolidated domain surface (Waves 2–4): agent adapter catalog +
        // Hermes providers, served directly from the ported Rust crates.
        .route("/api/adapters", get(api_adapters))
        // Health + build identity (deploy chain: CI stamps TRIOS_BUILD_SHA)
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/", get(health))
        // Rainbow Bridge (L13 / INV-8) — see crates/trios-rainbow-bridge.
        .merge(rainbow_routes::rainbow_routes())
        // A2A REST + SSE — wire-compatible with the Swift A2ARegistryClient
        // (TS-retirement item 5: the last Hono surface moved to Rust).
        .merge(rest_a2a::router())
        // BrowserOS local-state surface (Wave 6 / TS retirement): memory,
        // soul, skills, ACL rules, credits proxy, provider probe, monitoring.
        .merge(rest_browseros::router())
        // Agent tool-loop (Wave 8): /agent/run, /agent/run/stream, /agent/tools
        // — the Rust port of the retired TS agent core.
        .merge(rest_agent::router())
        // Sidepanel chat (Wave 16): POST /chat — AI SDK UI message stream,
        // the route the BrowserOS panel actually talks to. Re-implemented in
        // Rust after it silently died with the TS server.
        .merge(rest_chat::router())
        // Observability (Wave 11): Prometheus text exposition for the SR-03
        // browser command queue + server gauges.
        .merge(metrics::router())
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(axum::middleware::from_fn(security::origin_guard))
                .layer(axum::middleware::from_fn(security::auth_middleware))
                .layer(axum::middleware::from_fn(security::timeout_middleware)),
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(
        "trios-server {} ({}) listening on 0.0.0.0:{}",
        env!("CARGO_PKG_VERSION"),
        option_env!("TRIOS_BUILD_SHA").unwrap_or("dev"),
        port
    );
    info!("  WS:  ws://0.0.0.0:{}/ws", port);
    info!("  SSE: http://0.0.0.0:{}/sse  (Claude Desktop / Cursor)", port);
    info!("  REST: http://0.0.0.0:{}/api/chat", port);
    info!("  A2A: http://0.0.0.0:{}/a2a/*  (Swift registry client)", port);
    info!("  Agent loop: http://0.0.0.0:{}/agent/run", port);
    info!("  MCP tools: {} registered", tools::count());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

/// Build identity for the deploy chain. `TRIOS_BUILD_SHA` is stamped at
/// compile time by CI (release build step); local builds report "dev".
async fn version() -> Json<Value> {
    Json(json!({
        "name": "trios-server",
        "version": env!("CARGO_PKG_VERSION"),
        "git_sha": option_env!("TRIOS_BUILD_SHA").unwrap_or("dev"),
    }))
}

/// Adapter catalog + Hermes provider mappings, sourced from the consolidated
/// `trios-agent-harness` and `trios-openclaw` crates. Proves the single Rust
/// entry point reaches the ported domain logic (replaces the TS
/// `/api/agents/adapters` surface).
async fn api_adapters() -> Json<Value> {
    let adapters = trios_agent_harness::adapter_catalog();
    let hermes: Vec<Value> = trios_openclaw::SUPPORTED_PROVIDER_TYPES
        .iter()
        .filter_map(|p| {
            trios_openclaw::get_mapping(p).map(|m| {
                json!({ "providerType": p, "mapping": m })
            })
        })
        .collect();
    Json(json!({
        "adapters": adapters,
        "hermesProviders": hermes,
        "gatewayContainerPort": trios_openclaw::OPENCLAW_GATEWAY_CONTAINER_PORT,
    }))
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
