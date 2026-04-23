# TASK — BR-OUTPUT (trios-a2a)

## Status: DONE ✅

## Completed

- [x] `A2ARouter` struct with `SharedRegistry`
- [x] `A2ARouter::new()` — empty registry
- [x] `A2ARouter::registry()` — clone Arc for external access
- [x] `A2ARouter::call()` — dispatches 6 MCP tool calls:
  - `a2a_list_agents`
  - `a2a_send`
  - `a2a_broadcast`
  - `a2a_assign_task`
  - `a2a_task_status`
  - `a2a_update_task`
- [x] Tests: register + list, assign + status + update, unknown tool

## Open P0 (критично для системы)

- [ ] **`a2a_register_agent` tool** — сейчас регистрация только через прямой доступ к registry;
  нужен MCP tool `a2a_register_agent` чтобы агенты могли самостоятельно регистрироваться
- [ ] **`a2a_heartbeat` tool** — агенты должны сообщать что живы; без heartbeat
  Orchestrator не знает кто онлайн

## Open P1

- [ ] Add `a2a_get_messages` dispatch — получить входящие сообщения для агента
- [ ] Add `a2a_list_tasks` dispatch — список задач по статусу
- [ ] Add `A2ARouter::handle_mcp_request()` — принимает raw MCP JSON Request,
  парсит tool + params, возвращает MCP JSON Response (для прямой интеграции с trios-server)

## Open P2

- [ ] Add authentication stub — verify caller AgentId matches session
- [ ] Add rate limiting per agent (in-memory token bucket)
- [ ] Add `A2ARouter::snapshot()` — dump full registry state as JSON (for debug)

## Blocked by

Nothing. P0 items можно начинать прямо сейчас.
