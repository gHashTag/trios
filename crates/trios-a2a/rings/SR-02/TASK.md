# TASK — SR-02 (trios-a2a)

## Status: DONE ✅

## Completed

- [x] `A2ARegistry` struct: agents (HashMap), tasks (HashMap), messages (Vec)
- [x] `register_agent()` → `{ok, agent_id}`
- [x] `list_agents()` → JSON array
- [x] `send_message(from, to, payload)` → `{ok, message_id}`
- [x] `broadcast(from, payload)` → `{ok, message_id, recipients}`
- [x] `assign_task(title, created_by, assign_to)` → `{ok, task_id}`
- [x] `task_status(task_id)` → Task JSON
- [x] `update_task(task_id, new_state)` → `{ok, task_id, state}`
- [x] `SharedRegistry = Arc<Mutex<A2ARegistry>>`
- [x] `shared_registry()` constructor
- [x] `mcp_tool_definitions()` → 4 MCP tool schemas
- [x] Tests: register, send, assign, lifecycle

## Open

- [ ] Add `get_agent(agent_id)` → Option<AgentCard>
- [ ] Add `list_tasks(filter: TaskState)` → filtered list
- [ ] Add `get_messages(agent_id)` → messages for agent
- [ ] Add `a2a_task_status` MCP tool definition (currently in BR-OUTPUT only)
- [ ] Add `a2a_update_task` MCP tool definition
- [ ] Add agent heartbeat: `update_agent_status(agent_id, status)`
- [ ] Add message TTL / pruning for long-running registry

## Blocked by

Nothing.
