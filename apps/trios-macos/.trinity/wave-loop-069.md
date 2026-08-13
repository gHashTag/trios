# T27 Wave Loop - Plan WAVE-069

Domain: Hive self-improvement loop (`BR-OUTPUT/Hive*.swift`)
Context: WAVE-068 hardened the Hive core in this repo. This wave set out to
port the orchestrator, and found something that changes the whole picture.

## ANOMALY-069-1: there are two Hives, and the one that runs is the weaker one

`BR-OUTPUT/QueenTabView.swift` renders `EmbeddedQueenRoot` from **QueenUILib**,
the SwiftPM package at `TRINITY_ROOT/apps/queen` that `xtask/src/main.rs:148`
builds and links into `trios_app`. `EmbeddedQueenRoot` renders that package's
own `MainView`, with trios views injected at petal indices through
`QueenHostedRoute`.

Consequence: the Hive screen, its orchestrator, its bee runner and its
`Cmd+Shift+H` shortcut - all added to QueenUILib in an earlier cycle - **already
ship inside `trios_app`**. The claim recorded in WAVE-068 that "the loop cannot
run in this app" was wrong.

The two copies have diverged in opposite directions:

| | QueenUILib copy (runs in the app) | this repo's copy (CI-tested) |
|---|---|---|
| lines | 3,565 | 2,847 |
| orchestrator, bee runner, screen | yes | bee runner + dispatch (this wave) |
| chat-per-task | yes | no |
| stale-snapshot rejection | **no** | yes |
| confidence floor | **no** | yes |
| standing invariants | **no** | yes |
| anchor-free retry check | **no** | yes |
| worktree verifier fix | **no** | yes |
| Rust ring verifier | **no** | yes |
| tokenised test attribution | **no** | yes |
| build-artefact exclusion | **no** | yes |
| tests | 0 in this repo's CI | 224 |

**Every WAVE-068 safety fix is in the copy with no loop; the copy with the loop
has none of them.** Dependency direction forbids the obvious repair: QueenUILib
cannot import TriOSKit, so the shared core would have to live in QueenUILib.

Not acted on: the ledger scope set by the user confines a cycle to
`apps/trios-macos`, and no exception is recorded. Unification is a decision for
the operator, offered as option 1 at closeout.

## P0 (landed this wave)

- Bee runner ported, Foundation-only so CI compiles it
  -> `BR-OUTPUT/HiveBeeRunner.swift`
- Dispatch decision extracted as a pure function with 24 tests
  -> `BR-OUTPUT/HiveDispatch.swift`
- Outcome state machine pinned: a failing check overrides the bee's own claim
  of success; an unavailable check reaches review without counting as failure
- Anomaly recorded above

## P1 (next wave)

- Unify the two Hives (operator decision - see closeout options)
- `HiveOrchestrator` for this repo's copy, or retire this copy in favour of
  QueenUILib's with the hardening back-ported
- Register a Hive destination in `Trinity999TabMap` + `QueenTabView` hosted
  routes, the established pattern for every other trios surface

## P2

- Chat-per-task on the trios `ChatMessage` model
- `churn` normalises against the max; on a quiet repo one commit reads 1.0

## P3-P5

- `claude` CLI signed out; no bee has done real work in any tree
- `claude --worktree` with `-p` never observed end to end

## Literature takeaways (carried from WAVE-068, still governing)

- arXiv:2607.26117 - blind resampling beats self-repair; never hand a bee its
  own failed attempt. Encoded as `HiveInvariants.promptIsAnchorFree`.
- arXiv:2607.13083 - harness optimisers invent failures that never happened.
  Encoded as the dispatch confidence floor.
- Falsifiable Release Gates - a policy struct is a self-graded safety claim.
  Encoded as `HiveInvariants.check`, now consulted by `HiveDispatch.decide`
  before any dispatch rather than only in tests.

## Measured effect

| | WAVE-068 | WAVE-069 |
|---|---|---|
| tests | 191 | 224 |
| dispatch decision | untested, inside a view model | pure function, 24 tests |
| invariants | checked in tests only | consulted before every dispatch |
