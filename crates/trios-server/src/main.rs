mod mcp;
mod tools;

use axum::{
    routing::post,
    Router,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

const PORT: u16 = 9005;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let app = Router::new()
        .route("/mcp/tools/list", post(mcp::list_tools))
        .route("/mcp/tools/call", post(mcp::call_tool))
        .route("/health", axum::routing::get(health));

    let addr = format!("0.0.0.0:{PORT}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("trios-server v{} started on {addr}", env!("CARGO_PKG_VERSION"));
    info!("MCP endpoint: http://{addr}/mcp/tools/call");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
