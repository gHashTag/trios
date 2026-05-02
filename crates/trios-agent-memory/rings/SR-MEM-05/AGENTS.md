# AGENTS — SR-MEM-05

## Active soul

- `Loop-Locksmith` — author, contract design, mock backends, reverse
  `seed_replay` implementation.

## Constitutional rules honoured

- **R1** — pure Rust (no `.py` inside `crates/`, no `.sh`).
- **R5-honest** — concrete sqlx + Zig FFI deferred to BR-IO ring; documented in README §Honest scope.
- **R-RING-DEP-002** — Silver-tier deps only (`serde`, `serde_json`, `chrono`, `thiserror`, `tracing`, `tokio` runtime primitives, sibling SR-MEM rings).
- **R-L6-PURE-007** — no `.py` files inside this ring.
- **L1** — no `.sh` files in this ring.
- **L13** (I-SCOPE) — only this ring is touched by the bridge contract definitions.
- **L14** — every commit carries `Agent: Loop-Locksmith` trailer.
- **I5** — README, TASK, AGENTS, RING, Cargo.toml, src/lib.rs all present.
- **L21** — read-only forwarder on `lessons.rs`; `LessonsSource` trait
  has no `&mut self` method, so a downstream impl cannot smuggle in a
  write path through this contract.

## Anchor

`phi^2 + phi^-2 = 3`
