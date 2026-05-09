//! # trios-chat — Trinity Secure Chat (thin re-export shim)
//!
//! [VERIFIED] **R-RING-DEP-002 / L-ARCH-001:** This crate is a
//! re-export-only shim. All implementation lives in the ring stack
//! (`crates/trios-chat/rings/`). Binaries and external integrations
//! continue to use the historical `trios_chat::module::Item` paths;
//! the shim forwards them to `trios_chat_br_output`.
//!
//! ## Wiring
//! - Errors / `Result`     ← CR-CHAT-00 (via BR-OUTPUT-CHAT)
//! - `identity` / `sealed` ← CR-CHAT-01
//! - `ratchet`             ← CR-CHAT-02
//! - `group`               ← CR-CHAT-03
//! - `padding`             ← CR-CHAT-04
//! - `persist`             ← CR-CHAT-05 (trait) / BR-IO-CHAT-05 (impl)
//! - `capability` / `injection` ← CR-CHAT-06
//! - `r_chat`              ← CR-CHAT-LAWS
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! Parent EPIC: trinity-fpga#28 · Builds on: trinity-fpga#22 · trios#629.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Re-export every public item from the assembler ring so that legacy
// imports (`trios_chat::capability::CapabilityToken`, etc.) keep working
// without source-level edits.
pub use trios_chat_br_output::*;
