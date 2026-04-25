# RING — EXT-04 (trios-ext)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-ext-ext04 |
| Sealed | No |

## Purpose

BrowserOS A2A Agent ring. Executes browser control commands received from
trios-server via EXT-02 (MCP HTTP client). This is the "hands" of the
BrowserOS system — it can open tabs, navigate, read/write DOM, click, type.

## Why a new ring instead of extending EXT-03

EXT-03 is a **content injector** — it injects specific UI into specific sites
(GitHub button, Claude textarea). It's narrow and site-specific.

EXT-04 is a **general-purpose browser agent** — it executes arbitrary browser
commands from the A2A network. These are fundamentally different responsibilities.
Mixing them would violate R1 (single responsibility per ring).

## API Surface (pub + wasm_bindgen)

| Function | Chrome API used | Description |
|----------|-----------------|-------------|
| `browser_get_url()` | `window.location.href` | Current tab URL |
| `browser_get_title()` | `document.title` | Current tab title |
| `browser_navigate(url)` | `window.location.assign()` | Navigate to URL |
| `browser_get_dom()` | `document.documentElement.outerHTML` | Full page HTML |
| `browser_query_selector(sel)` | `document.querySelector` | Find element |
| `browser_click(sel)` | `.click()` | Click element |
| `browser_type(sel, text)` | `.value = text` + input event | Type into field |
| `browser_scroll(x, y)` | `window.scrollTo` | Scroll page |
| `browser_eval(js)` | `Function()` sandboxed | Eval JS expression |
| `dispatch_command(json)` | — | Route JSON command to above fns |

## Dependencies

- `wasm-bindgen`, `web-sys`, `js-sys`
- `serde`, `serde_json` (command deserialization)
- No imports from other EXT rings

## Laws

- R1: No imports from EXT-00, EXT-01, EXT-02, EXT-03
- L6: Pure Rust/WASM — no TypeScript
- `browser_eval` sandboxed — `new Function(js)()` only, no `eval()`
- All operations must be non-blocking (WASM single-threaded)
