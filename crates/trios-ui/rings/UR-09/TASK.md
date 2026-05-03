# TASK.md — UR-09

## Current
- [x] Create UR-09 ring structure
- [x] A2ASocialAtom GlobalSignal
- [x] A2ASocialMessage types (chat, interrupt, presence, system)
- [x] AgentProfile registry (BOS, Scarabs, Human, t27, System)
- [x] SocialFeed Dioxus component
- [x] PresenceBar Dioxus component
- [x] HumanInput Dioxus component
- [x] InterruptButton Dioxus component
- [x] HTTP polling via web-sys

## Next
- [ ] Add UR-09 to BR-APP Cargo.toml
- [ ] Add `Route::Social` to UR-08 AppShell
- [ ] Wire UR-07 ApiClient for WS-based social (replace HTTP poll)
- [ ] Build WASM + test in BRONZE-RING-EXT sidepanel
- [ ] Add Neon persistence (read history on load)
