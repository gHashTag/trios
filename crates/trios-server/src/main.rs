mod mcp;
mod security;
mod tools;

use axum::{
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tracing::info;
use tracing_subscriber::EnvFilter;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 9005;
const MAX_BODY_SIZE_BYTES: usize = 1024 * 1024; // 1 MB

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let mcp_service = mcp::McpService::new();

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/mcp/tools/list", post(list_mcp_tools))
        .route("/mcp/tools/call", post(call_mcp_tool))
        .layer(middleware::from_fn(security::timeout_middleware))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_SIZE_BYTES))
        .layer(middleware::from_fn(security::auth_middleware))
        .with_state(mcp_service);

    let host = std::env::var("TRIOS_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port: u16 = std::env::var("TRIOS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(
        "trios-server v{} started on {}",
        env!("CARGO_PKG_VERSION"),
        addr
    );
    info!("MCP endpoints:");
    info!("  - GET  /health");
    info!("  - POST /mcp/tools/list");
    info!("  - POST /mcp/tools/call");
    info!("Security:");
    info!(
        "  - Auth: {}",
        if std::env::var("TRIOS_API_KEY").map(|k| !k.is_empty()).unwrap_or(false) {
            "enabled (TRIOS_API_KEY set)"
        } else {
            "disabled (set TRIOS_API_KEY to enable)"
        }
    );
    info!(
        "  - Allowed roots: {}",
        std::env::var("TRIOS_ALLOWED_ROOTS")
            .map(|r| if r.is_empty() { "all (unrestricted)".into() } else { r })
            .unwrap_or_else(|_| "all (unrestricted)".into())
    );
    info!(
        "  - Request timeout: {}s",
        std::env::var("TRIOS_REQUEST_TIMEOUT_SECS")
            .ok()
            .unwrap_or_else(|| "30".into())
    );
    info!(
        "  - Max body size: {} bytes",
        MAX_BODY_SIZE_BYTES
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("trios-server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received Ctrl+C, shutting down..."),
        _ = terminate => info!("received SIGTERM, shutting down..."),
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "trios-server",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn list_mcp_tools(
    axum::extract::State(mcp_service): axum::extract::State<mcp::McpService>,
) -> Json<rust_mcp_schema::ListToolsResult> {
    Json(mcp_service.list_tools())
}

async fn call_mcp_tool(
    axum::extract::State(mcp_service): axum::extract::State<mcp::McpService>,
    Json(params): Json<rust_mcp_schema::CallToolRequestParams>,
) -> Json<rust_mcp_schema::CallToolResult> {
    Json(mcp_service.call_tool(params).await)
}
