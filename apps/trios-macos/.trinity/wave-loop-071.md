# T27 Wave Loop - Plan WAVE-071

Domain: Hive evidence handling
Context: five waves in, four of them adding to a copy of the Hive that does not
run. This wave starts by correcting two of my own measurements, then applies
two results that bear directly on evidence-gated agent loops.

## Two corrections to WAVE-069/070 findings

**`nm | grep -i hive` was a false positive.** It returned 2 matches, both
`SettingsScreen.autoArchiveDays` - "arc**hive**" contains "hive". And
`HiveScreen` is an internal Swift type, never an exported symbol, so `nm`
cannot answer whether the linked dylib carries the Hive in either direction.
The question remains open; the tool was wrong for it.

**The two Hives do not race.** They point at different repositories:

| | QueenUILib copy | this repo's copy |
|---|---|---|
| state file | `$TRINITY_ROOT/.trinity/queen/hive.json` | `apps/trios-macos/.trinity/hive/hive.json` |
| bees work in | `~/trinity` | `apps/trios-macos` |

WAVE-069 framed this as a file-level hazard. It is not. What survives is real
but narrower: **two independent daily budget ceilings**. Arm both and total
spend is twice whatever was set in either UI.

## P0 (landed)

- Evidence currency: verdicts carry the commit measured against; four states,
  only one of which reads as current -> `BR-OUTPUT/HiveVerifier.swift`,
  `BR-OUTPUT/HiveModels.swift`, `BR-OUTPUT/HiveRuntime.swift`
- STALE badge on a review card -> `BR-OUTPUT/HiveTabView.swift`
- Anti-gaming clause in the bee prompt, naming concrete cheats
  -> `BR-OUTPUT/HivePriorityEngine.swift`
- 11 tests -> `tests/TriOSKitTests/HiveEvidenceTests.swift`

## P1 (next wave)

- Unify the two Hives (operator decision, five waves outstanding)
- Sibling-budget awareness: read the other copy's `hive.json` and surface the
  combined exposure, so two ceilings cannot be mistaken for one
- Verify `cargo xtask build` carries the new tab

## P2

- Chat-per-task on the trios `ChatMessage` model
- `churn` normalises against the max; on a quiet repo one commit reads 1.0

## Literature takeaways

- **Proof-or-Stop: Don't Trust the Agent, Trust the Evidence - Loop Engineering
  for Verifiable Evidence-Gated Lifecycle Control.** Lifecycle states such as
  reviewed, tested and DONE "remain claims unless supported by current
  evidence". The Hive already gated review on an executed check; what it lacked
  was *currency*. Implemented this wave.
- **Autoresearch with Coding Agents: Generalizers and Metric-Maximizers on
  Quran Recitation Data.** An agent left alone with a dataset, an evaluation
  script and one editable file iterates without supervision - and the paper
  separates generalizers from metric-maximizers. The Hive hands each bee the
  exact signal that selected its task, which is the maximizer's ideal setup.
  Anti-gaming clause added.
- **The Role Specialization Model (RSM)** (arXiv:2608.12311). Explicit role
  coordination supports architectural quality but "requires deliberate
  coordination strategies, context management, and human verification of
  agent-generated outputs" - the review gate stays.
- **CyberLLM** - safety constraints forbid autonomous agents from acting
  without oversight; guarded response as a first-class state. Consistent with
  `blocked` being distinct from `idle`.
- **AgentCompass** - unified evaluation infrastructure; relevant if the Hive
  ever needs to compare bee performance across models.

## Measured effect

| | WAVE-070 | WAVE-071 |
|---|---|---|
| tests | 231 | 242 |
| verdict staleness | undetectable | four states, one current |
| metric gaming | unaddressed | forbidden by name in every prompt |
