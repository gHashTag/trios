# EXT-03 — Content Injectors

GitHub and Claude.ai content injectors. Pure Rust/WASM replacing deleted TypeScript files.

## API
### GitHub
- `github_parse_issue_number()` → `Option<u32>` — Parse issue/PR number from URL
- `github_inject_button()` → `Result<bool, JsValue>` — Inject Trinity button
- `github_injector_start()` — Entry point for content script

### Claude.ai
- `claude_find_textarea()` → `Option<HtmlTextAreaElement>` — Find ProseMirror textarea
- `claude_inject_text(text)` → `bool` — Inject text into Claude
- `claude_auto_submit()` → `bool` — Click submit button
- `claude_dispatch(text, auto_submit)` → `bool` — Inject + optionally submit
- `claude_injector_start()` — Entry point for content script (wasm_bindgen(start))

## Dependencies
- `trios-ext-00` — DOM `document()` for GitHub injector

## Usage
```rust
use trios_ext_03::{github_injector_start, claude_dispatch};

github_injector_start(); // Called from github-bootstrap.js
claude_dispatch("Hello agent", true); // Inject + auto-submit
```
