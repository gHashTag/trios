//! Raw FFI declarations for zig-agents C API.

use libc::{c_char, c_int, size_t};

extern "C" {
    /// Get Trinity version from zig-agents
    pub fn trinity_version() -> *const c_char;

    /// Send a collaboration message between agents
    pub fn trinity_collaboration_send(
        msg_ptr: *const u8,
        msg_len: size_t,
        timeout_ms: size_t,
    ) -> c_int;

    /// Get MCP server health status
    pub fn trinity_health_check() -> c_int;

    /// Deploy MCP server to Fly.io region
    pub fn trinity_deploy_fly(region: *const c_char, org: *const c_char) -> c_int;

    /// Get current instance status
    pub fn trinity_instance_status() -> *const c_char;

    /// Restart an instance
    pub fn trinity_restart_instance() -> c_int;

    /// Stop an instance
    pub fn trinity_stop_instance() -> c_int;

    /// Get TLS certificate status
    pub fn trinity_cert_status() -> *const c_char;

    /// Get agent collaboration status
    pub fn trinity_agent_status(agent_id: *const c_char) -> *const c_char;

    /// Register a new agent
    pub fn trinity_register_agent(
        agent_type: *const c_char,
        config: *const c_char,
    ) -> c_int;

    /// Unregister an agent
    pub fn trinity_unregister_agent(agent_id: *const c_char) -> c_int;

    /// Query agent registry
    pub fn trinity_list_agents() -> *const c_char;

    /// Spawn autonomous task
    pub fn trinity_spawn_task(
        task: *const c_char,
        agent_id: *const c_char,
    ) -> c_int;

    /// Get task status
    pub fn trinity_task_status(task_id: *const c_char) -> *const c_char;

    /// Cancel a task
    pub fn trinity_cancel_task(task_id: *const c_char) -> c_int;

    /// Clean free string returned from zig-agents
    pub fn trinity_free_string(ptr: *mut c_char);
}
