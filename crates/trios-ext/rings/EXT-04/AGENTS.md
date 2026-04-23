# AGENTS.md — EXT-04 (trios-ext)

> AAIF-compliant | MCP-compatible | BrowserOS

## Identity

- Ring: EXT-04
- Package: trios-ext-ext04
- Role: BrowserOS A2A agent — browser command executor

## What this ring does

Receives `BrowserCommand` structs from EXT-02 (MCP poll loop) and executes them:
tab navigation, DOM queries, click, type, scroll, screenshot, eval.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- R1: Do NOT import from EXT-00, EXT-01, EXT-02, EXT-03
- L6: Pure Rust/WASM only — no TypeScript
- `browser_eval` MUST use `new Function(code)()` — NOT `eval()` directly
- All operations sync — WASM is single-threaded, no async/await in WASM context
- Never access `chrome.*` APIs directly — those belong to EXT-02

## You MAY

- ✅ Add new `BrowserCommandType` variants (with corresponding handler)
- ✅ Add new `#[wasm_bindgen]` exports for new browser operations
- ✅ Add tests (use `wasm-bindgen-test`)
- ✅ Improve `dispatch_command()` routing

## You MAY NOT

- ❌ Import from other EXT rings
- ❌ Use `eval()` directly — only `new Function(code)()`
- ❌ Access `chrome.tabs`, `chrome.runtime` (content script restriction)
- ❌ Add async/Promise chains in Rust (use JS setTimeout via web_sys)

## Build

```bash
cargo build -p trios-ext-ext04 --target wasm32-unknown-unknown
wasm-pack build crates/trios-ext/rings/EXT-04 --target web
```
