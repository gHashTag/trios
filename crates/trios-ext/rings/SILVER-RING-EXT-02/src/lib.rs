//! SILVER-RING-EXT-02 — MCP Client + Background Service
//!
//! Provides MCP WebSocket client and Chrome extension background service worker.
//! Depends on SILVER-RING-EXT-01 (DOM bridge) for UI updates.

pub mod bg;
pub mod mcp;

pub use bg::background_init;
pub use mcp::{
    mcp_connect, mcp_is_connected, mcp_list_agents, mcp_list_tools, mcp_ping, mcp_send_chat,
    McpClient, McpRequest, MCP_WS_URL,
};
