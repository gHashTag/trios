mod mcp;
mod mcp_endpoints;
mod operator;
mod security;
mod tools;
mod ws_handler;

use axum::Router;
use axum::routing::get;
use std::net::SocketAddr;
use tower::ServiceBuilder;
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

    let app = Router::new()
        .route("/ws", get(ws_handler::ws_handler))
        .route("/operator", get(operator::operator_ws_handler))
        .route("/health", get(health))
        .route("/", get(health))
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(security::auth_middleware))
                .layer(axum::middleware::from_fn(security::timeout_middleware)),
        )
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("trios-server listening on ws://0.0.0.0:{}/ws", port);
    info!("Operator bridge: ws://0.0.0.0:{}/operator?token=...", port);
    info!("MCP tools: {count} registered", count = tools::count());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
