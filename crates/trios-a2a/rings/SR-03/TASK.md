# TASK — SR-03 (trios-a2a)

## Status: ACTIVE

## Completed ✅

- [x] `BrowserCommandType` enum — все 12 операций
- [x] `BrowserCommand` struct — команда с id, type, params, TTL
- [x] `BrowserResult` struct — результат с ok/data/error
- [x] `BrowserCommandQueue` — очередь с deduplication
- [x] `mcp_browser_tool_definitions()` — 12 MCP tool schemas
- [x] `AgentCard::browser_agent(tab_id)` в SR-00

## Open P0 (нужен trios-server)

- [ ] **HTTP endpoint** `GET /mcp/browser-commands`
  trios-server держит `BrowserCommandQueue` из SR-03,
  возвращает pending команды extension'у
- [ ] **HTTP endpoint** `POST /mcp/browser-result`
  принимает `BrowserResult`, обновляет очередь
- [ ] **MCP handler** `a2a_browser_command` — AI агент вызывает tool,
  команда попадает в очередь

## Open P1 (trios-ext WASM)

- [ ] EXT-02: `poll_commands()` → GET /mcp/browser-commands (каждые 2s)
- [ ] EXT-02: `report_result()` → POST /mcp/browser-result
- [ ] EXT-02: `register_agent()` → POST /a2a с AgentCard
- [ ] EXT-02/03: `dispatch_command(json)` — роутер к DOM операциям

## Open P2

- [ ] `browser_screenshot` — html2canvas или getDisplayMedia
- [ ] `browser_wait_for(selector, timeout_ms)` — MutationObserver
- [ ] WebRTC stream для live view (далёкое будущее)
