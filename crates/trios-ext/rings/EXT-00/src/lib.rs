//! EXT-00 — Shell & Transport
//!
//! Sidepanel UI builder (DOM) + MCP client (HTTP transport).
//! These two modules are tightly coupled: the MCP client renders
//! responses directly into the DOM panels.

pub mod dom;
pub mod mcp;

pub use dom::*;
pub use mcp::*;
