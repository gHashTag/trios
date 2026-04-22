# EXT-02 — Settings

chrome.storage.local wrapper for API keys and preferences.

## API
- `get_api_key()` → `Option<String>` — Get cached API key
- `save_api_key(key)` → `Result<(), JsValue>` — Save to chrome.storage.local
- `load_api_key()` — Load from chrome.storage.local (call once on startup)
- `settings_save_key(key)` — wasm_bindgen export

## Dependencies
None (standalone ring).

## Usage
```rust
use trios_ext_02::{load_api_key, save_api_key, get_api_key};

load_api_key(); // Call once on startup
if let Some(key) = get_api_key() { /* use key */ }
```
