---
name: agent-safe-build
description: Build trios-macos without breaking the app the user is running. Use for any build, rebuild, verification or release - especially when several agents share the repository. Covers the xtask interface, the dev/release split, and the verification traps that made earlier reports wrong.
---

# Agent-safe build

## The rule

**This repository has no shell scripts.** Law L1 is enforced in CI: a single
`.sh` anywhere fails Constitutional Enforcement. Everything the old scripts did
lives in `xtask`, a Rust binary.

```bash
cargo run -p trios-app-xtask --bin trios-app -- build
cargo run -p trios-app-xtask --bin trios-app -- chat-sse-e2e
cargo run -p trios-app-xtask --bin trios-app -- mesh-chat-e2e
cargo run -p trios-app-xtask --bin trios-app -- e2e-flow
```

If you find yourself writing a `.sh`, the answer is a new xtask subcommand.

## What is isolated

The dev and release variants share nothing:

| Axis | dev | release |
|------|-----|---------|
| Bundle | `trios-dev.app` | `trios.app` |
| Bundle id | `com.browseros.trios.dev` | `com.browseros.trios` |
| Binary | `trios_dev_app` | `trios_app` |
| Frameworks | `Frameworks-dev` | `Frameworks` |
| Data root | `.trinity-dev` | `.trinity` |
| MCP port | 9205 | 9105 |
| Secrets | `DevSecretStore` (files) | Keychain |

The data root matters most: while it was shared, a dev schema change could
corrupt the running app's encrypted database.

## Which sources get compiled

`xtask build` takes every git-tracked Swift file plus `BR_OUTPUT_ALLOWLIST`, a
short list of untracked prototypes that the app genuinely depends on. Adding a
new view therefore needs no build change once it is committed - but an untracked
draft will not compile, which is deliberate: BR-OUTPUT doubles as a scratch area
and half-finished files must not break the app.

## Verification traps

These produced confidently wrong reports. Check for them.

1. **Do not grep for failure text.** A crash traps before printing anything, so
   "zero FAIL lines" reads as success. Assert on the positive signal and check
   the exit code.
2. **A check that silently matches nothing is indistinguishable from a check
   that passes.** Run every new scanner against a known-bad input before
   believing a clean result. The self-audit scanner once matched `func Queen...`
   when only types carry that prefix, found nothing, and reported health.
3. **Never print a metric you did not measure.** `nil` and `0` are different
   claims. A missing usage figure shown as "0 tokens", or a 580-character skill
   shown as "0k chars", turns an instrumentation gap into a statement about the
   thing being measured.
4. **Clean a write-fixture before the run, not only after.** Otherwise the first
   run of the day passes and every later one fails, which reads as flake.
5. **A test double cannot test the layer it stands in for.** A replayed stream
   proves the parser and everything above it; it cannot prove anything the
   server does.
6. **Replacing a fixed structure invalidates every test that indexed it.** After
   plans became dynamic, `items[1]` crashed. Grep the suite for literal indices
   into anything you just made variable-length.

## Known limits

- The XCTest target under `tests/TriOSKitTests/` has a large pre-existing
  breakage: missing types and Swift 6 actor-isolation errors. Judge the app by
  the build and by `chat-sse-e2e`, which is where the real assertions live.
- The Xcode license can block `swiftc` with no warning. Workaround:
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
