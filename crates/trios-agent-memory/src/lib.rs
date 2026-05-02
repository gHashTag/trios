//! trios-agent-memory — GOLD IV crate facade.
//!
//! Re-exports SR-MEM-00 typed primitives for every downstream GOLD IV
//! ring (SR-MEM-01 kg-client-adapter, SR-MEM-05 episodic-bridge,
//! BR-OUTPUT AgentMemory trait assembler).
//!
//! L-RING-FACADE-001: this file MUST NOT contain business logic.

pub use trios_agent_memory_sr_mem_00::*;
