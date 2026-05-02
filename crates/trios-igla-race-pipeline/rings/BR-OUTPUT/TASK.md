# TASK — BR-OUTPUT IglaRacePipeline

Closes #459 · Part of #446 · Soul: `Loop-Locksmith`

## Acceptance

- [x] `crates/trios-igla-race-pipeline/rings/BR-OUTPUT/` with I5 trinity (README.md, TASK.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs)
- [x] Deps: SR-00..04 path deps + serde/chrono/thiserror (Bronze-tier, NO I/O)
- [x] Public API: `IglaRacePipeline::new(cfg)` / `with_gardener(cfg, g)`, `run_e2e_ttt_o1(sinks, trainer) -> Result<VictoryReport, PipelineErr>`
- [x] Implementation: per seed → `Scarab::queued → Running → chunks → Done|Pruned`, then `check_victory`
- [x] Falsifier tests verbatim ported into `tests/victory_falsifiers.rs` (18 tests)
- [x] Integration test: 1 fake seed, end-to-end loop, < 60s, BPB row written, ASHA prune decision recorded
- [x] `R-RING-FACADE-001`: parent `src/lib.rs` is 28 LoC, re-exports only
- [x] PR closes #459 with `Agent: Loop-Locksmith` trailer

## Out of scope (R5-honest, deferred)

- [ ] `crates/trios-igla-race/src/lib.rs ≤ 50 LoC` — follow-up PR. Current facade re-exports 11 internal modules used by 4 bins (`main.rs`, `seed_emit.rs`, `qk_gain_check.rs`, `ledger_check.rs`). Shrinking requires porting `asha`, `invariants`, `hive_automaton`, `neon`, `race`, `rungs`, `attn`, `ema`, `sampler`, `status`, `victory` modules into pipeline-side rings — a separate refactor.
- [ ] Concrete sqlx `BpbSink` / `GardenerSink` adapters → BR-IO ring (after SR-02 #454)
- [ ] Real `PythonTrainer` backend → BR-IO ring (after SR-02 #454)
- [ ] GPU sweep producing `val_bpb < 1.07063` → follow-up PR via SR-02
