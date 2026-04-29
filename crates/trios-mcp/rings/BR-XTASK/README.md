# BR-XTASK: Launcher

## Ring Purpose

Single command `cargo run -p trios-mcp` to launch all rings in parallel.
Replaces `package.json` scripts from browser-tools-mcp (bun dev).

## TypeScript Reference

`package.json` scripts → `bun dev` command

## Rust Files

- `src/main.rs` — Single launch command, parallel execution

## Dependencies (workspace)

- tokio, anyhow, tracing
- tracing-subscriber — Logging init
- Ring dependencies via path:
  - `trios-mcp-sr00 = { path = "../SR-00" }`
  - `trios-mcp-sr01 = { path = "../SR-01" }`
  - `trios-mcp-sr02 = { path = "../SR-02" }`

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MCP_HOST` | `127.0.0.1` | Server host |
| `MCP_PORT` | `3025` | Server port (law L5) |
| `RUST_LOG` | `trios_mcp=info,warn` | Log filter |

## Parallel Launch

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    // Parse configuration
    let host = env::var("MCP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("MCP_PORT")
        .unwrap_or_else(|_| "3025".to_string())
        .parse()?;

    // Launch SR-00 in background
    tokio::spawn(async move {
        trios_mcp_sr00::BrowserServer::run(host.clone(), port).await
    });

    // Launch SR-02 in foreground (stdio for MCP)
    trios_mcp_sr02::McpServer::run_stdio().await?;
}
```

## Ring Status

- [x] `cargo run -p trios-mcp` launches everything
- [x] SR-00 and SR-02 run in parallel
- [x] Environment variables parsed correctly
- [x] Ctrl+C graceful shutdown
- [x] `RING.md` present (R3)
- [x] Separate `Cargo.toml` (R2)
- [x] Tests pass (R4, ≥10 tests)
