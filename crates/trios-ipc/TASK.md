# TASK.md — trios-ipc

## P0 — Protocol Core

- [x] `Cargo.toml` — serde + serde_json + uuid only
- [x] `src/envelope.rs` — MessageEnvelope, RingId, PROTOCOL_VERSION
- [x] `src/payload.rs` — IpcPayload with all variants
- [x] `src/types.rs` — AgentInfo, McpToolInfo, BrowserCommandReq
- [x] `src/validate.rs` — validate(), validate_route(), IpcError
- [x] `RING.md` + `AGENTS.md` + `TASK.md` present
- [x] Registered in workspace Cargo.toml

## P0 — Tests

- [x] validate() returns Err(VersionMismatch) on version=0
- [x] validate() returns Err(MissingId) on empty id
- [x] validate() returns Err(UnauthorizedRoute) on forbidden route
- [x] Valid routes pass validation
- [x] Serialize/deserialize roundtrip
- [x] `cargo clippy -p trios-ipc -- -D warnings` clean

## P1 — Integration (Next PR)

- [ ] `trios-ui/Cargo.toml` depends on `trios-ipc`
- [ ] `trios-ext/Cargo.toml` depends on `trios-ipc`
- [ ] `trios-server/Cargo.toml` depends on `trios-ipc`
- [ ] UR-07 uses MessageEnvelope for chrome.runtime.sendMessage()
- [ ] EXT-02 uses MessageEnvelope for onMessage handler
