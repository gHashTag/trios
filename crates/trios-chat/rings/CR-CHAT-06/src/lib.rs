//! # CR-CHAT-06 — capability + injection
//!
//! Two tightly-coupled "guardrail" rings that defend the agent from
//! prompt-injection and over-broad tool access. Both implement the
//! safety half of Trinity Chat's threat model.
//!
//! - [`capability`] — signed, session-scoped capability tokens
//!   (R-CHAT-6/8) + signed tool manifests. **INV-CHAT-2**
//!   (`agent action set ⊆ capability.scope`).
//! - [`injection`] — dual-LLM input classifier + deterministic
//!   deny-list output validator (R-CHAT-7).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod capability;
pub mod injection;

pub use capability::{CapError, CapabilityToken, Scope, ToolManifest};
pub use injection::{classify_input, quarantine_wrap, validate_output, InjectionError, TaggedSpan, Trust};
