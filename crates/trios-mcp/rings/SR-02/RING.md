# SR-02: MCP stdio Server

## Ring Purpose

MCP (Model Context Protocol) stdio server with 14 tools for browser automation.
Replaces `browser-tools-mcp/mcp-server.ts`.

## TypeScript Reference

`browser-tools-mcp/mcp-server.ts` — MCP stdio protocol with 14 tools

## Rust Files

- `src/lib.rs` — MCP server entry point
- `src/protocol.rs` — MCP JSON-RPC over stdio
- `src/tools.rs` — All 14 MCP tools
- `src/discovery.rs` — Server discovery (ports 3025-3035)

## Dependencies (workspace)

- tokio, serde, serde_json, anyhow, tracing
- reqwest — HTTP client for SR-00 calls

## MCP Tools (14 total)

### Basic Browser Tools (4)

| # | Tool | Method | Endpoint | Description |
|---|-------|--------|----------|-------------|
| 1 | `getConsoleLogs` | GET | `http://host:port/console-logs` | Check browser console logs |
| 2 | `getConsoleErrors` | GET | `http://host:port/console-errors` | Check browser console errors |
| 3 | `getNetworkErrors` | GET | `http://host:port/network-errors` | Check network ERROR logs (status ≥ 400) |
| 4 | `getNetworkLogs` | GET | `http://host:port/network-success` | Check ALL network logs (success + errors) |

### Tools with Complex Logic (3)

| # | Tool | Method | Endpoint | Description |
|---|-------|--------|----------|-------------|
| 5 | `getSelectedElement` | GET | `http://host:port/selected-element` | Get currently selected DOM element |
| 6 | `wipeLogs` | POST | `http://host:port/wipelogs` | Clear all browser logs from memory |
| 7 | `takeScreenshot` | POST | `http://host:port/capture-screenshot` | Take screenshot of current browser tab |

### Audit Tools (4)

| # | Tool | Method | Endpoint | Description |
|---|-------|--------|----------|-------------|
| 8 | `runAccessibilityAudit` | POST | `http://host:port/accessibility-audit` | Run Lighthouse accessibility audit |
| 9 | `runPerformanceAudit` | POST | `http://host:port/performance-audit` | Run Lighthouse performance audit |
| 10 | `runSeoAudit` | POST | `http://host:port/seo-audit` | Run Lighthouse SEO audit |
| 11 | `runBestPracticesAudit` | POST | `http://host:port/best-practices-audit` | Run Lighthouse best practices audit |

### Special Tools (3)

| # | Tool | Description |
|---|-------|-------------|
| 12 | `runNextJSAudit` | Comprehensive Next.js SEO audit (static checklist) |
| 13 | `runDebuggerMode` | Guide systematic debugging process (8-step workflow) |
| 14 | `runAuditMode` | Automated optimization workflow (all audits in sequence) |

## JSON-RPC over stdio

```rust
pub struct McpServer {
    host: String,
    port: u16,
    auth: AuthConfig,
}

impl McpServer {
    pub async fn run_stdio(&self) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin).lines();
        let mut writer = BufWriter::new(stdout);

        loop {
            let line = reader.next_line().await?;
            let request: JsonRpcRequest = serde_json::from_str(&line)?;
            let response = self.handle_request(request).await?;
            writeln!(writer, "{}", response)?;
        }
    }
}
```

## Server Discovery

- Check ports 3025-3035 sequentially
- `GET /.identity` endpoint with signature `"mcp-browser-connector-24x7"`
- Auto-reconnect on failure

## Ring Status

- [ ] JSON-RPC over stdio working
- [ ] All 14 MCP tools implemented
- [ ] Tools call SR-00 HTTP endpoints correctly
- [ ] Server discovery (ports 3025-3035) working
- [ ] Tool registration complete
- [ ] `RING.md` present (R3)
- [ ] Separate `Cargo.toml` (R2)
- [ ] Tests pass (R4, ≥10 tests)
