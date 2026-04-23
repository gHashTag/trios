# TASK — EXT-04 (trios-ext)

## Status: ACTIVE — начинаем сейчас

## Open P0 (минимально для работы)

- [ ] **`BrowserCommand` struct** — deserialized MCP command:
  ```rust
  pub struct BrowserCommand {
      pub id: String,        // UUID
      pub tool: String,      // "browser_navigate", "browser_click", ...
      pub params: Value,     // serde_json::Value
  }
  ```
- [ ] **`BrowserResult` struct** — result to report back:
  ```rust
  pub struct BrowserResult {
      pub command_id: String,
      pub ok: bool,
      pub data: Value,
      pub error: Option<String>,
  }
  ```
- [ ] **`dispatch_command(json: &str) -> String`** — парсит BrowserCommand, роутит, возвращает BrowserResult JSON
- [ ] **`browser_get_url()`** — `window.location.href`
- [ ] **`browser_get_title()`** — `document.title`
- [ ] **`browser_navigate(url: &str)`** — `window.location.assign(url)`
- [ ] **`browser_get_dom() -> String`** — `document.documentElement.outerHTML`
- [ ] **`browser_query_selector(sel: &str) -> Option<String>`** — outer HTML элемента

## Open P1

- [ ] `browser_click(selector: &str) -> bool`
- [ ] `browser_type(selector: &str, text: &str) -> bool` + input/change events
- [ ] `browser_scroll(x: f64, y: f64)`
- [ ] `browser_eval(js: &str) -> String` — `new Function(js)()` sandboxed

## Open P2

- [ ] `browser_screenshot()` — `html2canvas` или `getDisplayMedia` (requires permissions)
- [ ] `browser_wait_for(selector, timeout_ms)` — MutationObserver-based
- [ ] `browser_get_text(selector)` — `textContent` элемента

## Blocked by

EXT-02 P0: `mcp_poll_commands()` — без поллинга EXT-04 не будет получать команды.
