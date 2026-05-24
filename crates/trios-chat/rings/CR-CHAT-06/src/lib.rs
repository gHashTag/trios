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
pub mod confused_deputy;
pub mod injection;
pub mod session_capability_replay;
pub mod tag_stripping;
pub mod tool_arg_confusion;
pub mod tool_output_sanitization;
pub mod tool_response_size_bound;
pub mod injection_pattern_depth_guard;

pub use capability::{CapError, CapabilityToken, Scope, ToolManifest};
pub use confused_deputy::{check_invocation, DeputyError, Invocation, NonceLedger};
pub use injection::{classify_input, quarantine_wrap, validate_output, InjectionError, TaggedSpan, Trust};
pub use session_capability_replay::{
    validate_session_cap, SessionCapError, SessionCapToken, SessionCapView, SESSCAP_MAX_TTL_SECS,
};
pub use tag_stripping::{
    parse_structured_output, serialise_structured_output, Span, SpanTag, TagSplit,
};
pub use tool_output_sanitization::{
    sanitize_tool_output, ToolOutputError, TOUT_MAX_LEN, TOUT_NESTED_SENTINEL,
};
pub use tool_response_size_bound::{
    validate_tool_response_size, ToolResponseSizeError, TRSB_MAX_BYTES, TRSB_MAX_LINE_LEN,
    TRSB_MAX_LINES,
};
pub use injection_pattern_depth_guard::{
    count_nesting_depth, validate_injection_depth, InjectionDepthError, INJECTION_SENTINEL,
    IPDG_MAX_DEPTH, IPDG_MAX_INPUT_LEN,
};

pub mod tool_cot_leak_guard;
pub use tool_cot_leak_guard::{
    validate_no_cot_leak, CotLeakError, TCOT_MAX_LEN,
};

pub mod capability_scope_escalation_guard;
pub use capability_scope_escalation_guard::{
    validate_scope_history, validate_scope_transition, ScopeEscalationError, ScopeSnapshot,
    CSEG_MAX_CHANGES,
};

pub mod tool_call_chain_depth_guard;
pub use tool_call_chain_depth_guard::{
    validate_tool_chain, ChainStep, ToolChainError, TCCD_MAX_DEPTH, TCCD_MAX_TOTAL_INPUT,
};
pub mod agent_output_rate_limit_guard;
pub use agent_output_rate_limit_guard::{
    validate_output_rate, OutputRateError, OutputEvent,
    AORL_MAX_BYTES, AORL_MAX_OUTPUTS, AORL_MAX_WINDOW_MS, AORL_MIN_INTERVAL_MS,
};

pub mod tool_argument_schema_guard;
pub use tool_argument_schema_guard::{
    validate_tool_args, SchemaArgValue, PropertyDef, SchemaError, SchemaType,
    TASG_MAX_PROPS, TASG_MAX_STRING_LEN,
};

pub mod agent_context_window_budget_guard;
pub use agent_context_window_budget_guard::{
    validate_context_budget, ContextEntry, ContextBudgetError,
    ACWB_MAX_BUDGET, ACWB_MAX_ENTRIES,
};

pub mod tool_response_timeout_guard;
pub use tool_response_timeout_guard::{
    validate_tool_timeout, ToolTimeoutError, TRTO_MAX_TIMEOUT_MS, TRTO_MIN_TIMEOUT_MS,
};

pub use tool_arg_confusion::{
    validate_tool_call, ArgKind, ArgSpec, ArgValue, ToolCall, ToolCallError, ToolEntry,
    ToolManifest as ToolArgManifest, NESTED_TOOL_CALL_SENTINEL,
};
