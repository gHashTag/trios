# T27 Wave Loop - Plan WAVE-070

Domain: Hive self-improvement loop, surface and runtime
Context: WAVE-069 found that two copies of the Hive exist and that the one
running inside `trios_app` (QueenUILib's) carries none of the safety work.
Unification needs an operator decision that has not arrived, so this wave takes
the in-scope option: give the tested copy a real surface in this repo.

## P0 (landed)

- `HiveRuntime` - timer, child processes, persisted state; every decision
  delegated to the pure `HiveDispatch` -> `BR-OUTPUT/HiveRuntime.swift`
- `HiveTabView` - the workspace, `TriosTheme` colours and `ProjectPaths` only,
  per L6 -> `BR-OUTPUT/HiveTabView.swift`
- `.hive` destination at petal 15, shortcut 7 -> `rings/SR-00/Trinity999TabMap.swift`
- Hosted route registration -> `BR-OUTPUT/QueenTabView.swift`
- xtask allowlist entry -> `xtask/src/main.rs`

## ANOMALY-070-1: a standalone test that fails, reported twice as passing

`tests/swift/trinity_999_tab_map_test.swift` asserted "Seven Trios workspaces
must be hosted" against a map holding eight. It has been failing since `skills`
was added at petal 3.

Nothing runs it. `.github/workflows/trios-swift.yml` compiles four standalone
tests - `todo_list_projection`, `todo_panel_policy`, `chat_workspace_layout`,
`chat_logic` - and not this one. Meanwhile two files under `.claude/plans/`
record `trinity_999_tab_map_test.swift standalone - pass`.

Fixed twice over:

1. The standalone now asserts against `Trios999Destination.allCases` rather
   than a hard-coded count, which is the drift that let it rot.
2. The same assertions live in `tests/TriOSKitTests/Trinity999TabMapTests.swift`,
   where `swift test` runs them on every CI run. `Trinity999TabMap` is inside
   the CI slice (`rings/SR-00`), so there was never a reason for the assertions
   to sit somewhere unexecuted.

Added while there: an unassigned petal must keep its canonical Queen screen, so
a future route cannot silently displace one.

## P1 (next wave)

- Unify the two Hives (still an operator decision)
- Retire the standalone tab-map test in favour of the CI one, or add it to the
  workflow - two copies of the same assertions will drift again
- Verify `cargo xtask build` produces a bundle carrying the new tab

## P2

- Chat-per-task on the trios `ChatMessage` model
- `churn` normalises against the max; on a quiet repo one commit reads 1.0

## P3-P5

- `claude` CLI signed out; no bee has done real work in any tree

## Literature takeaways

Carried unchanged from WAVE-068; all three still govern and are now enforced at
runtime rather than only in tests, since `HiveDispatch.decide` consults
`HiveInvariants.check` before any dispatch.

- arXiv:2607.26117 - blind resampling beats self-repair. `HiveRuntime.dispatch`
  passes the original prompt on every attempt, never `lastError`.
- arXiv:2607.13083 - harness optimisers invent failures that never happened.
  A declined target is now *recorded* (`target_declined`) rather than dropped,
  so the operator can see what the Queen refused to work on and why.
- Falsifiable release gates - invariant violations are surfaced in the policy
  panel, not only in the test suite.

## Measured effect

| | WAVE-069 | WAVE-070 |
|---|---|---|
| tests | 224 | 231 |
| Hive surface in this repo | none | petal 15, shortcut 7 |
| tab map covered by CI | no | yes |
| standalone tab-map test | failing, unrun | passing |
