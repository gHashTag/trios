# trios MCP Infrastructure Status

> Updated: 2026-04-23

## Транспорты

| Транспорт | URL | Статус | Клиенты |
|-----------|-----|--------|----------|
| **SSE** | `http://localhost:9005/sse` | ✅ | Claude Desktop, Cursor, VSCode |
| **WebSocket** | `ws://localhost:9005/ws` | ✅ | Агенты, trios-http |
| **REST** | `http://localhost:9005/api/chat` | ✅ | curl, внешние клиенты |
| **SSE (remote)** | `https://playras-macbook-pro-1.tail01804b.ts.net/sse` | ✅ | Remote MCP clients |

## .roo/mcp.json

```json
{
  "mcpServers": {
    "trios": { "url": "http://localhost:9005/sse" },
    "trios-remote": { "url": "https://playras-macbook-pro-1.tail01804b.ts.net/sse" }
  }
}
```

## tools/list — 26 инструментов

### Git/FS/KG (19)
`git_log`, `git_diff`, `git_status`, `git_commit`, `git_branch`, `git_checkout`,
`git_push`, `git_pull`, `fs_read`, `fs_write`, `fs_list`, `fs_delete`,
`fs_mkdir`, `kg_query`, `kg_update`, `kg_entities`, `kg_relations`,
`search_code`, `run_tests`

### A2A (7)
`a2a/register`, `a2a/list_agents`, `a2a/send`, `a2a/broadcast`,
`a2a/assign_task`, `a2a/task_status`, `a2a/update_task`

## A2A endpoint тесты

| Метод | Запрос | Результат |
|-------|--------|-----------|
| `a2a/register` | `{id:"alpha", name:"Alpha Agent"}` | `{"ok":true}` ✅ |
| `a2a/register` | `{id:"beta", name:"Beta Agent"}` | `{"ok":true}` ✅ |
| `a2a/list_agents` | `{}` | 2 agents ✅ |
| `a2a/send` | `{from:"alpha", to:"beta", payload:{...}}` | `{"ok":true, "message_id":"..."}` ✅ |
| `a2a/assign_task` | `{title:"Build SR-02", assign_to:"beta"}` | `{"ok":true, "task_id":"..."}` ✅ |

## Тест-покрытие

| Крейт | Тестов | Статус |
|-------|--------|--------|
| `trios-a2a` | 16/16 | ✅ |
| `trios-server` | 26/26 | ✅ |
| **Итого** | **42/42** | ✅ |

## Запуск

```bash
cargo run -p trios-server
# trios-server listening on 0.0.0.0:9005
# SSE: http://0.0.0.0:9005/sse  (Claude Desktop / Cursor)
# WS:  ws://0.0.0.0:9005/ws
# REST: http://0.0.0.0:9005/api/chat
# MCP tools: 26 registered
```
