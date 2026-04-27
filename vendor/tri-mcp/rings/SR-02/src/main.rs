//! SR-02 — MCP stdio Server Entry Point

use trios_mcp_sr02::run_stdio_loop;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_loop().await
}
