# BR-OUTPUT — IglaRacePipeline assembler 🥉

**Soul-name:** `Loop-Locksmith` · **Codename:** `DELTA` · **Tier:** 🥉 Bronze · **Kingdom:** Cross-kingdom

> Closes #459 · Part of #446
> Anchor: `φ² + φ⁻² = 3`

## Mission

Tie SR-00..04 into the canonical `IglaRacePipeline::run_e2e_ttt_o1()` end-to-end loop and expose the INV-7 victory gate (verbatim port of the legacy `trios-igla-race::victory` module — the Popper-razor falsifiers travel with the assembler so the gate cannot be silently bypassed).

## Honest scope (R5)

| Concern | This PR | Where it lands |
|---|---|---|
| Assembler API + integration loop | ✅ | here |
| INV-7 victory gate (verbatim port) | ✅ | here (`tests/victory_falsifiers.rs` — 18 falsifiers) |
| `MockedTrainer` deterministic backend | ✅ | here |
| Real `PythonTrainer` over SR-02 | ❌ | BR-IO ring (after SR-02 #454 merges) |
| Concrete sqlx `BpbSink` / `GardenerSink` | ❌ | BR-IO ring |
| `trios-igla-race::lib.rs` ≤ 50 LoC shrink | ❌ | follow-up PR (current 68-LoC facade is used by 4 internal bins; shrink requires porting 11 internal modules into pipeline-side rings) |

## Surface

```rust
let cfg = PipelineCfg {
    strategy_id: StrategyId::new(),
    worker_id: WorkerId::new("acc1", 0),
    seeds: vec![Seed(1597), Seed(2584), Seed(4181)],
    steps_per_chunk: 250,
    total_steps: INV2_WARMUP_BLIND_STEPS as i64 + 1_000,
    trainer_config: serde_json::json!({}),
};
let mut pipeline = IglaRacePipeline::new(cfg);
let report = pipeline.run_e2e_ttt_o1(
    Sinks { bpb: &mut sink, gardener: &mut gd_sink },
    &mut MockedTrainer::winning(),
).await?;
```

Per seed: `Scarab::queued → Running → (chunk × N: trainer → writer → gardener) → Done | Pruned`. Final BPB rows are fed to `check_victory`. `Ok(VictoryReport)` iff 3+ distinct seeds clear `BPB_VICTORY_TARGET = 1.5` strictly. Otherwise `Err(PipelineErr::HonestNotYet { passing, required })`.

## Tests — 27/27 GREEN

| Suite | Count |
|---|---|
| Lib unit tests | 6 |
| Integration (`tests/integration.rs`) | 3 |
| Victory falsifiers (`tests/victory_falsifiers.rs`, verbatim port) | 18 |

```
# integration.rs
integration_one_fake_seed_completes_under_60s
integration_three_seeds_reach_victory
integration_losing_curve_returns_honest_not_yet
```

`cargo clippy -p trios-igla-race-pipeline-br-output --all-targets -- -D warnings` → **0 warnings**.

## Constitutional compliance

- **R-RING-FACADE-001** ✓ outer `trios-igla-race-pipeline/src/lib.rs` is now 28 LoC, re-exports only
- **R-RING-DEP-002** ✓ Bronze-tier deps: SR-00..04 path deps + `serde`/`chrono`/`thiserror`. Dev-only `tokio` for tests. NO sqlx/reqwest/subprocess
- **R-RING-BR-004** ✓ Bronze ring re-exposed via parent GOLD I facade
- **R-L6-PURE-007** ✓ no `.py` here; trainer trait-gated and stubbed
- **L13** ✓ single-ring scope
- **I5** ✓ README.md, TASK.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs all present
- **L14** ✓ `Agent: Loop-Locksmith` trailer

## Pre-existing CI honest disclosure

Pre-existing failures on `main` (`I5` red on legacy `crates/trios-a2a/rings/*` and `crates/trios-mcp/rings/*`, `ARCH-UI`, generic `Test`) are out of scope. Merge via `--admin --squash --delete-branch` per EPIC #446 stewardship.
