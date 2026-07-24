//! trios-agent-harness — agent lifecycle backend.
//!
//! Re-export facade only (L-ARCH-001). Ported from the TS backend
//! `lib/agents/*` during Wave 2 of the Rust consolidation.
//!
//! Rings:
//! - AH-00 `core`  — AgentDefinition, history, stream events (data + serde)
//! - AH-01 `catalog` — adapter descriptors, defaults, lookup
//! - AH-02 `queue` — bounded per-agent FIFO message queue
//! - AH-03 `turns` — RingBuffer + TurnRegistry state machine

pub use trios_agent_harness_ah00 as core;
pub use trios_agent_harness_ah01 as catalog;
pub use trios_agent_harness_ah02 as queue;
pub use trios_agent_harness_ah03 as turns;

// Flat re-exports of the most-used items.
pub use trios_agent_harness_ah00::{
    AgentAdapter, AgentDefinition, AgentState, AgentStatus, AgentStreamEvent, HistoryEntry,
    HistoryRole, HistoryToolCall, PermissionMode, TextStream, ToolCallStatus,
};
pub use trios_agent_harness_ah01::{catalog as adapter_catalog, descriptor_for, AdapterDescriptor};
pub use trios_agent_harness_ah02::{MessageQueue, QueueFullError, QueuedMessage};
pub use trios_agent_harness_ah03::{ActiveTurnInfo, RingBuffer, TurnRegistry, TurnStatus};
