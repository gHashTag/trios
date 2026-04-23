# RING.md — trios-ipc

> Ring: IPC Protocol | Metal: Bronze (cross-cutting) | Version: 1

## Purpose

Typed IPC contract between `trios-ui` (Dioxus WASM sidepanel) and `trios-ext` (Chrome Extension WASM).
Pure `serde` types — no `wasm-bindgen`, no `web-sys`, no `dioxus`.

## Protocol

Messages are `MessageEnvelope` serialized to JSON, sent via `chrome.runtime.sendMessage()`.

### Allowed Routes

| From | To | Direction |
|------|----|-----------|
| UR-07 (ApiClient) | EXT-02 (Background) | UI → Extension |
| EXT-02 (Background) | UR-07 (ApiClient) | Extension → UI |
| EXT-02 (Background) | EXT-01 (DOM) | Background → Content Script |
| EXT-02 (Background) | SV-03 (Server) | Extension → Server HTTP |

### Versioning

`PROTOCOL_VERSION` must be incremented on any breaking change to `IpcPayload`.

## Files

| File | Role |
|------|------|
| `envelope.rs` | `MessageEnvelope`, `RingId`, `PROTOCOL_VERSION` |
| `payload.rs` | `IpcPayload` enum (all message variants) |
| `types.rs` | Shared data types (`AgentInfo`, `McpToolInfo`, `BrowserCommandReq`) |
| `validate.rs` | `validate()`, route validation, `IpcError` |
