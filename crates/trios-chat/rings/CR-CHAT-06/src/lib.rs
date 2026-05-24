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

pub mod agent_prompt_injection_depth_guard;
pub use agent_prompt_injection_depth_guard::{
    validate_injection_depth as validate_prompt_depth,
    InjectionDepthError as PromptDepthError,
    RepromptEntry,
    PIDP_MAX_DEPTH, PIDP_MAX_REPROMPT_LEN, PIDP_MAX_TOTAL_BYTES,
};

pub mod agent_tool_call_audit_log_guard;
pub use agent_tool_call_audit_log_guard::{
    validate_audit_log, AuditEntry, AuditLogError,
    TCAL_MAX_ENTRIES,
};

pub mod agent_output_content_type_validation_guard;
pub use agent_output_content_type_validation_guard::{
    validate_content_types, ContentType, ContentTypeError, ToolOutput,
    OCTV_MAX_MISMATCHES, OCTV_MAX_PAYLOAD, OCTV_MIN_PAYLOAD,
};

pub mod agent_tool_call_rate_limit_guard;
pub use agent_tool_call_rate_limit_guard::{
    validate_tool_call_rate, RateLimitError,
    ATCR_MAX_CALLS, ATCR_MAX_CALL_IDS, ATCR_WINDOW_MS,
};

pub mod agent_tool_input_sanitization_guard;
pub use agent_tool_input_sanitization_guard::{
    validate_tool_inputs, SanitizationError, ToolInput,
    ATIS_MAX_INPUT_LEN, ATIS_MAX_INPUTS,
};

pub mod agent_tool_output_size_validation_guard;
pub use agent_tool_output_size_validation_guard::{
    validate_tool_output_sizes, OutputSizeError, ToolOutputRecord,
    ATOS_MAX_ACCUMULATED, ATOS_MAX_OUTPUTS, ATOS_MAX_SINGLE,
};

pub mod agent_response_length_bound_guard;
pub use agent_response_length_bound_guard::{
    validate_response_lengths, AgentResponse, ResponseLengthError,
    ARLB_MAX_BATCH, ARLB_MAX_CUMULATIVE, ARLB_MAX_RESPONSE_LEN, ARLB_ID_LEN,
};

pub mod agent_session_concurrency_limit_guard;
pub use agent_session_concurrency_limit_guard::{
    validate_session_concurrency, ConcurrencyError, SessionRecord,
    ASCL_MAX_BATCH, ASCL_MAX_CONCURRENT, ASCL_MAX_PER_USER, ASCL_MAX_PRIORITY,
    ASCL_MIN_PRIORITY, ASCL_SESSION_ID_LEN, ASCL_USER_ID_LEN,
};

pub mod agent_tool_authorization_scope_guard;
pub use agent_tool_authorization_scope_guard::{
    validate_tool_authorization, ScopeAuthError, ToolInvocation,
    ATAS_MAX_RECORDS, ATAS_MAX_SCOPE_DEPTH, ATAS_SESSION_ID_LEN,
};

pub mod agent_prompt_injection_detection_rate_guard;
pub use agent_prompt_injection_detection_rate_guard::{
    validate_detection_rate, DetectionRateError, DetectionSample,
    PIDR_MAX_FP_RATE, PIDR_MAX_RECORDS, PIDR_MIN_RATE, PIDR_MIN_SAMPLES,
};

pub mod agent_output_redaction_completeness_guard;
pub use agent_output_redaction_completeness_guard::{
    validate_redaction, RedactionCheck, RedactionError,
    AORC_MAX_OUTPUTS, AORC_MIN_REDACT_LEN, AORC_OUTPUT_ID_LEN,
};

pub mod agent_session_timeout_enforcement_guard;
pub use agent_session_timeout_enforcement_guard::{
    validate_session_timeouts, SessionTimeout, SessionTimeoutError,
    ASTE_MAX_SESSIONS, ASTE_MAX_TIMEOUT_MS, ASTE_MIN_TIMEOUT_MS, ASTE_SESSION_ID_LEN,
};

pub mod agent_resource_usage_limit_guard;
pub use agent_resource_usage_limit_guard::{
    validate_resource_limits, ResourceLimitError, ResourceRecord,
    ARUL_MAX_CPU_MS, ARUL_MAX_DISK, ARUL_MAX_MEMORY, ARUL_MAX_RECORDS, ARUL_SESSION_ID_LEN,
};

pub mod agent_tool_call_dependency_cycle_guard;
pub use agent_tool_call_dependency_cycle_guard::{
    validate_no_cycles, DependencyCycleError, DependencyEdge,
    ATDC_MAX_EDGES, ATDC_TOOL_ID_LEN,
};

pub mod agent_output_format_validation_guard;
pub use agent_output_format_validation_guard::{
    validate_output_format, FormatValidationError, OutputRecord,
    AOFV_APPROVED_TYPES, AOFV_MAX_LEN, AOFV_MAX_OUTPUTS, AOFV_OUTPUT_ID_LEN, AOFV_TEXT_FORBIDDEN,
};

pub mod agent_tool_result_cache_staleness_guard;
pub use agent_tool_result_cache_staleness_guard::{
    validate_cache_staleness, CacheEntry, CacheStalenessError,
    ATRC_ENTRY_ID_LEN, ATRC_HASH_LEN, ATRC_MAX_AGE_MS, ATRC_MAX_ENTRIES,
};

pub mod agent_context_window_overflow_guard;
pub use agent_context_window_overflow_guard::{
    validate_context_overflow, ContextOverflowError, ContextWindowEntry,
    ACWO_ENTRY_ID_LEN, ACWO_MAX_BUDGET, ACWO_MAX_ENTRIES, ACWO_MAX_PRIORITY,
};

pub mod agent_prompt_injection_depth_accumulation_guard;
pub use agent_prompt_injection_depth_accumulation_guard::{
    validate_injection_depth_accum, InjectionDepthAccumError, RepromptDepthRecord,
    APID_MAX_CUMULATIVE_DEPTH, APID_MAX_ENTRIES, APID_MAX_SINGLE_DEPTH, APID_SESSION_ID_LEN,
};

pub mod agent_tool_call_frequency_burst_guard;
pub use agent_tool_call_frequency_burst_guard::{
    validate_tool_call_burst, BurstError, ToolCallBurst,
    ATCF_MAX_BURST, ATCF_MAX_ENTRIES, ATCF_MAX_WINDOW_MS, ATCF_SESSION_ID_LEN,
};

pub use tool_arg_confusion::{
    validate_tool_call, ArgKind, ArgSpec, ArgValue, ToolCall, ToolCallError, ToolEntry,
    ToolManifest as ToolArgManifest, NESTED_TOOL_CALL_SENTINEL,
};
