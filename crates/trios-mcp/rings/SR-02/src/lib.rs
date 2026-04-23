//! # SR-02: MCP stdio Server
//!
//! MCP (Model Context Protocol) stdio server with 14 tools for browser automation.
//! Replaces `browser-tools-mcp/mcp-server.ts`.

pub mod discovery;
pub mod protocol;
pub mod tools;

use anyhow::{Context, Result};
use tokio::io::{stdin, stdout, BufReader, BufWriter};
use tracing::{info, warn};

/// MCP server for stdio communication
pub struct McpServer {
    pub host: String,
    pub port: u16,
}

impl McpServer {
    /// Create new MCP server
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Run MCP server on stdio
    pub async fn run_stdio(&self) -> anyhow::Result<()> {
        info!("📡 MCP stdio server starting on {}:{}", self.host, self.port);

        let mut reader = BufReader::new(stdin());
        let mut writer = BufWriter::new(stdout());

        loop {
            let line = reader.read_line().await?;
            if line.trim().is_empty() {
                continue;
            }

            // Handle JSON-RPC request
            match protocol::handle_jsonrpc(&line, self.host.clone(), self.port) {
                Ok(response) => {
                    writeln!(writer, "{}", response)?;
                    writer.flush()?;
                }
                Err(e) => {
                    warn!("Failed to handle request: {}, error: {}", line, e);
                }
            }
        }
    }

    /// Get server identifier
    pub const IDENTITY: &'static str = "trios-mcp-server-v1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_new() {
        let server = McpServer::new("127.0.0.1", 3025);
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 3025);
    }

    #[test]
    fn test_identity() {
        assert_eq!(McpServer::IDENTITY, "trios-mcp-server-v1");
    }
}
