# RING — CR-CHAT-06 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-06` |
| Path    | `crates/trios-chat/rings/CR-CHAT-06/` |
| Sealed  | No |

## Purpose

The agent-safety ring — two co-located guardrail modules:

1. **`capability`** — signed, session-scoped capability tokens
   (R-CHAT-6/8) + signed tool manifests. **INV-CHAT-2**
   (`agent action set ⊆ capability.scope`).
2. **`injection`** — deterministic deny-list output validator +
   quarantine sandwich for untrusted input spans (R-CHAT-7).

They share Ed25519, SHA-256, and the same threat model (prompt
injection that tries to escalate scope or forge a manifest), so
keeping them in one ring avoids cross-ring duplication of the deny
list / signature plumbing.

## Public API

| Item | Module | Role |
|---|---|---|
| `Scope` | capability | Enum of allowed agent actions |
| `CapabilityToken` | capability | Signed, ttl-bound token |
| `ToolManifest` | capability | Signed schema-hash binding |
| `CapError` | capability | Verification error |
| `Trust`, `TaggedSpan` | injection | Input classification |
| `classify_input` / `quarantine_wrap` / `validate_output` | injection | Filter primitives |
| `InjectionError` | injection | Validator error |

## Dependencies

| Dep | Why |
|---|---|
| `trios-chat-cr-chat-00` | (re-exported via top-level `Error`/`Result` if downstream wants it) |
| `serde` + `serde_json` | Wire format + canonical scope serialization |
| `thiserror`     | Local error enums |
| `ed25519-dalek` | Token + manifest signatures |
| `sha2`          | Manifest signing-bytes hash |
| `rand_core`     | Token nonce |

No async, no I/O.

## Invariants

- **R-CHAT-6** — every `ToolManifest` is signed; `verify()` is the only
  way an orchestrator should ingest a tool.
- **R-CHAT-7** — output validator rejects 49+ canonical
  injection/deny phrases. Length cap: 32 KiB.
- **R-CHAT-8** — `CapabilityToken::issue` panics if `ttl_secs > 3600`.
- **INV-CHAT-2** — `verify` rejects when `required ∉ scopes`.

## Tests

11 unit tests (6 capability + 5 injection).

## Sibling Bronze

None — pure logic. Persistence of issued tokens is a Bronze concern
handled by BR-IO-CHAT-05 (under the generic envelope row); no Bronze
sibling specific to CR-CHAT-06 today.

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
