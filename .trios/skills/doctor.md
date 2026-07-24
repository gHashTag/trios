---
name: doctor
description: Comprehensive health check for the woody-weed-bot Rust/Dioxus/Telegram Mini App project
tags: [health-check, rust, dioxus, deploy, monitoring]
---

# /doctor — Project Health Monitor

Run a full diagnostic sweep over the codebase and deployment artifacts. Return a markdown report with per-check status and a short remediation summary.

## When to invoke

Run `/doctor` when:
- A deploy broke something
- Before committing UI/game changes
- After any `trunk build` or `dist/` update
- Periodically to catch silent regressions

## Checks performed

| # | Check | Tool | Severity |
|---|-------|------|----------|
| 1 | Native library compiles | `cargo check --lib` | blocker |
| 2 | WASM frontend compiles | `cargo check --target wasm32-unknown-unknown` | blocker |
| 3 | Unit tests pass | `cargo test --lib` | blocker |
| 4 | Code formatting clean | `cargo fmt --check` | warn |
| 5 | Production frontend builds | `trunk build` | blocker |
| 6 | `dist/` is in sync with sources | hash matching | warn |
| 7 | No references to missing functions | grep for undefined patterns | blocker |
| 8 | Game UI regression patterns | static scan of `src/ui/game/shop_game.rs` | warn |

## Procedure

On invocation, execute all checks sequentially. For each check:
- If it succeeds → **OK**
- If it can be fixed automatically or needs attention → **WARN**
- If it blocks deploy or runtime → **FAIL**

After all checks, print a markdown table plus a 2–3 line summary of the most important findings.

## Report format

```markdown
## 🩺 Project Health Report

| Check | Status | Details |
|-------|--------|---------|
| Native compile | ✅ OK | ... |
| WASM compile | ❌ FAIL | ... |
| ... | ... | ... |

### Summary
- 1 blocker: ...
- 1 warning: ...
```

## Static game regressions to flag

In `src/ui/game/shop_game.rs`, warn if any of these are detected:
1. `TableState::Seated` / `Eating` without `{ .. }` or `{ vip }` — pattern mismatch after schema change
2. `rand_order()` called inside a render component (e.g. `TableRow`) — causes flicker
3. `combo_timer_ms` is assigned but never decremented — combo never resets
4. `LocalStorage::get` deserialization without a fallback to `ShopState::new()` — schema drift crash
5. Function name referenced but no `fn` definition found in the same file — missing helper

## Deployment artifact check

- List `dist/woody-weed-bot-*.js` and `dist/woody-weed-bot-*_bg.wasm`
- Read `dist/index.html` and extract the JS/WASM hashes it references
- If the referenced files do not exist or there are multiple/stale artifacts → WARN

## Notes

- Keep total runtime under ~3 minutes; run unit tests last since they are the slowest after trunk.
- Do not auto-fix findings unless the user explicitly asks after the report.
- If `trunk` is missing, report WARN with install instructions.
