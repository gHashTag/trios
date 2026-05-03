# AGENTS.md — UR-09

## Agent: ALPHA
- Implement `A2ASocialAtom` GlobalSignal
- Implement `SocialFeed` Dioxus component
- Wire HTTP polling via `gloo-net` or `web-sys`

## Agent: BETA
- Add UR-09 to BR-APP dependencies
- Add `Route::Social` to UR-08 router
- Test WASM build + sidepanel load

## Rules
- R1: This ring depends on UR-00 (atoms), UR-01 (tokens), UR-02 (primitives)
- R2: No direct DOM manipulation — use Dioxus rsx! only
- R3: HTTP fetch via web-sys, not raw JS interop
- R4: All bus URLs configurable via SettingsAtom.mcp_url
