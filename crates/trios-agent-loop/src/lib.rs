//! trios-agent-loop — agent tool-loop backend.
//!
//! Re-export facade only (L-ARCH-001). Ported from the TS agent core
//! `packages/agent-core/src/agent/*` during Wave 8 of the Rust consolidation.
//!
//! Rings:
//! - AL-00 `llm`   — OpenAI-compatible chat contract + HTTP client (LlmClient)
//! - AL-01 `tools` — Tool trait, ToolRegistry, builtin + browser tools (BW-01 bridge)
//! - AL-02 `loop`  — AgentLoop state machine, step events, stop conditions

pub use trios_agent_loop_al00 as llm;
pub use trios_agent_loop_al01 as tools;
pub use trios_agent_loop_al02 as agent_loop;

pub use trios_agent_loop_al00::{
    AssistantTurn, ChatMessage, ChatRequest, HttpLlmClient, LlmClient, LlmConfig, ToolCallRequest,
    ToolDef,
};
pub use trios_agent_loop_al01::{
    register_browser_tools, register_builtin_tools, BrowserBridge, Tool, ToolRegistry,
};
pub use trios_agent_loop_al02::{AgentLoop, AgentLoopConfig, AgentRunResult, StepEvent, StopReason};
