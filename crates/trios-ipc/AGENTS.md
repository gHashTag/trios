# AGENTS.md — trios-ipc

> AAIF-compliant | Pure Rust IPC types

## Identity

- Crate: trios-ipc
- Metal: Bronze (cross-cutting)
- Role: Typed IPC contract between trios-ui and trios-ext

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- NO `wasm-bindgen`, `web-sys`, `dioxus` dependencies
- ONLY `serde`, `serde_json`, `uuid` allowed
- Increment `PROTOCOL_VERSION` on breaking changes
- Every recipient MUST call `envelope.validate()` before processing

## You MAY

- Add new `IpcPayload` variants
- Add new shared types to `types.rs`
- Add new `RingId` variants
- Extend allowed routes in `validate.rs`

## You MAY NOT

- Add async/tokio
- Add wasm-specific dependencies
- Add business logic (this is types only)
- Duplicate types that exist here in other crates
