# TASK — EXT-02 (trios-ext)

## Status: IN PROGRESS

## Completed

- [x] `chrome.storage.local` wrapper: `save_api_key`, `load_api_key`, `get_api_key`
- [x] `settings_save_key()` wasm_bindgen export

## Open P0 (критично для BrowserOS)

- [ ] **`a2a_register()`** — POST /a2a `{tool: "a2a_register_agent", params: {id, name, capabilities}}`
  Через `web_sys::fetch_with_request` (без WebSocket)
- [ ] **`mcp_poll_commands()`** — GET `{server_url}/mcp/browser-commands`
  возвращает `Vec<BrowserCommand>` (deserialized JSON)
- [ ] **`mcp_report_result()`** — POST `{server_url}/mcp/browser-result`
  принимает `BrowserResult` struct
- [ ] **`start_poll_loop()`** — `setInterval` 2000ms → `mcp_poll_commands()` → dispatch в EXT-04
  (через `js_sys::Function` callback)

## Open P1

- [ ] `get_server_url()` — читать `trios_server_url` из chrome.storage.local
- [ ] `get_poll_interval_ms()` — читать `poll_interval_ms` из storage (default 2000)
- [ ] Retry backoff: 2s → 4s → 8s → max 30s при ошибках сервера
- [ ] Command deduplication по `command_id` чтобы не выполнять дважды

## Open P2

- [ ] Аутентификация: передавать `zai_key` в header `Authorization: Bearer {key}`
- [ ] Логирование команд в `chrome.storage.local` с TTL 1h
