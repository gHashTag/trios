# Trinity Agent Skills — committed archive

This directory is the canonical archive of agent skills used in the gHashTag Trinity ecosystem (trios, trios-railway, trios-trainer-igla, t27).

Single source of truth: each skill ships as `.trinity/skills/<name>/SKILL.md`. The Perplexity Skills Library (https://www.perplexity.ai/computer/skills) mirrors this content.

## Index

| Skill | Purpose |
|---|---|
| [`autonomous-research-loop`](autonomous-research-loop/SKILL.md) | Long-running uninterrupted multi-step execution (research, code, tests, commits). Trigger: "work autonomously", "don't stop", "I'm going to sleep", etc. |
| [`leaderboard-snapshot`](leaderboard-snapshot/SKILL.md) | Manual IGLA-race leaderboard from primary artefacts (seed_results.jsonl, ALPHA comments, Audit Watchdog, Railway probe, Neon `bpb_samples`). Trigger: `/leaderboard`, "live leaderboard". |
| [`nasa-mission-report`](nasa-mission-report/SKILL.md) | Format any verification, deploy, audit, or smoke-test as a NASA-style mission report (Document ID, Verification Matrix, As-Flown, ICA, GO/NO-GO). |
| [`t27-phi-loop`](t27-phi-loop/SKILL.md) | PHI LOOP protocol for the t27 Trinity ternary computing repo (https://github.com/gHashTag/t27). Edit spec → seal hash → gen → test → verdict → save → commit. |
| [`trainer-igla-sot`](trainer-igla-sot/SKILL.md) | TRAINER-IGLA-SOT mission ONE SHOT — 5-lane migration (L-T1..L-T5) of the trainer pipeline into the SoT repo `trios-trainer-igla`. |
| [`tri-gardener-runbook`](tri-gardener-runbook/SKILL.md) | **v2.0 master runbook** — covers tri-gardener Rust orchestrator, EPIC #446 ring-refactor, ONE-SHOT v2.0 dispatch (7 codenames, anti-collision), NEON `ssot.chapters` (44 chapters), R14 batch, App.K Agent Memory, INV-13 born-Proven policy. |

Anchor: `phi^2 + phi^-2 = 3` · DOI 10.5281/zenodo.19227877 (B007 HSLM Benchmark Corpus, **not** root anchor).

## When to update this archive

- A skill is added or modified in the personal Skills Library → mirror to this directory in the same PR.
- A skill triggers, frontmatter, or core logic changes → bump `metadata.version` and update the index above.
- Never edit a `SKILL.md` here without also updating the personal library — this is a mirror, not an alternate fork.

## Links

- EPIC #446 — https://github.com/gHashTag/trios/issues/446
- ONE-SHOT v2.0 — https://github.com/gHashTag/trios/issues/236
- LAWS.md v2.0 — ../../LAWS.md
- AGENTS.md — ../../AGENTS.md
- CLAUDE.md — ../../CLAUDE.md
