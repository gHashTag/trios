# T27 Wave Loop - Plan WAVE-068

Domain: Hive self-improvement loop (`BR-OUTPUT/Hive*.swift`)
Context: WAVE-067 landed the Hive core ported from the superseded Trinity Queen
package (`cd3eedd`). This wave audits that code, applies recent literature on
self-improving agent harnesses, and hardens the guardrails.

## P0 (Critical / landed this wave)

- Verifier resolved a worktree to the repository root, then looked for
  `Package.swift` there -> every worktree bee returns UNVERIFIED
  -> `BR-OUTPUT/HiveVerifier.swift`
- `parseIssueMentions` built `prefix/segment` keys for flat roots, so
  `BR-OUTPUT/...` mentions scored against a key no module has
  -> `BR-OUTPUT/HiveRepoScanner.swift`
- Dead `modulePrefixes` binding in `scan()`
  -> `BR-OUTPUT/HiveRepoScanner.swift`
- Confidence floor before dispatch -> `BR-OUTPUT/HiveInvariants.swift`,
  `BR-OUTPUT/HivePriorityEngine.swift`
- Standing invariants, machine-checked -> `BR-OUTPUT/HiveInvariants.swift`
- Anchor-free retry check -> `BR-OUTPUT/HiveInvariants.swift`
- Real issues snapshot (45 open issues via `gh`) -> `.trinity/issues_snapshot.json`

## P1 (High / next wave)

- Port `HiveOrchestrator` and a Hive tab beside `QueenTabView`; without them
  the loop cannot run in this app -> `BR-OUTPUT/`, `xtask/src/main.rs`
  (`BR_OUTPUT_ALLOWLIST`)
- Wire the invariant check into the cycle so a violation halts dispatch

## P2 (Medium)

- Chat-per-task on the trios `ChatMessage` model
- `churn` normalises against the max, so on a quiet repo one commit reads 1.0

## P3-P5 (Backlog / research)

- `claude` CLI is signed out; no bee has done real work in any tree
- `claude --worktree` with `-p` never observed end to end
- Per-module status declaration - deliberately NOT invented this wave

## Literature takeaways

- **Try Again, Don't Look Back: Blind Resampling Outperforms Self-Repair in
  Small Code Models** (arXiv:2607.26117). Returning a failed program to the
  model anchors it: 33-68% of retries reproduce a near-identical program,
  against 2-14% under blind resampling. Execution feedback added nothing
  measurable over a content-free placebo below 7B. The Hive's retry is already
  a fresh session with the original prompt; `promptIsAnchorFree` makes that a
  tested property rather than an accident.
- **Phantom Guardrails: When Self-Improving Agent Harnesses Fix Failures That
  Never Happened** (arXiv:2607.13083). A proposer editing an agent's scaffold
  enabled a guardrail for a failure class that provably never occurred in 15/60
  runs, citing violations a byte-exact oracle refuted, triggered by input that
  merely resembled a familiar rule. Directly motivates the confidence floor:
  work proposed against evidence never gathered is the same failure.
- **Falsifiable Release Gates for Self-Improving Systems: Standing Invariants
  at Scale**. Safety claims for self-improving runtimes are almost always
  self-graded - a policy file, a guardrail, a promise in a README. `HivePolicy`
  is precisely such a file; `HiveInvariants.check` is the falsifiable version.
- **From Failed Trajectories to Reliable LLM Agents: Diagnosing and Repairing
  Harness Flaws**. Harness quality, not model quality, bounds agent
  reliability - supports investing in the verifier and the scanner over prompt
  tuning.
- **Live-SWE-agent: Can Software Engineering Agents Self-Evolve on the Fly?**
  Background for the P1 orchestrator work.

## Measured effect

| | before | after |
|---|---|---|
| module confidence | 68% | 88% |
| top target | `BR-OUTPUT` (size 28045) | `rings/SR-02` (36 open issues) |
| dispatchable targets | n/a | 21 of 21 |
| invariant violations | not checked | 0 |
| tests | 177 | 191 |
