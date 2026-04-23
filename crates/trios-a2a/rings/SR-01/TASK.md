# TASK — SR-01 (trios-a2a)

## Status: DONE ✅

## Completed

- [x] `A2AMessage` envelope: id (UUID), from/to AgentId, msg_type, payload (JSON), timestamp (RFC3339)
- [x] `A2AMessageType` enum: Direct, Broadcast, TaskAssign, TaskUpdate, TaskResult, Heartbeat, Error
- [x] `A2AMessage::direct()` factory
- [x] `A2AMessage::broadcast()` factory
- [x] `Task` struct: id, title, description, assigned_to, created_by, state, priority, timestamps
- [x] `TaskState` enum: Pending → Assigned → InProgress → Completed/Failed/Cancelled
- [x] `TaskPriority` enum with Ord
- [x] `Task::new()`, `Task::assign_to()`, `Task::is_terminal()`
- [x] Tests: message creation, broadcast, task lifecycle, serialization

## Open

- [ ] Add `Task::with_description()` builder
- [ ] Add `Task::with_priority()` builder
- [ ] Add `Task::start()` → InProgress transition
- [ ] Add `Task::complete()` / `Task::fail()` terminal transitions
- [ ] Add `A2AMessage::heartbeat(from)` factory
- [ ] Add `A2AMessage::task_result(from, to, task_id, result)` factory
- [ ] Add message signature/verification stub (for future auth)

## Next ring: SR-02
