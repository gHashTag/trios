# BR-OUTPUT — AlgorithmArena assembler 🥉

**Soul-name:** `Arena-Anchor` · **Codename:** `BETA` · **Tier:** 🥉 Bronze · **Kingdom:** Cross-kingdom

> Closes #460 · Part of #446
> Anchor: `φ² + φ⁻² = 3`

## Honest scope (R5)

This ring is the **assembler** — the place every downstream caller actually goes through to register specs and trigger runs. It is honest about two limits:

1. **Trainer is mocked.** Until SR-02 `TrainerRunner` ships, the only `TrainerBackend` impl is `MockedTrainer`. It returns deterministic synthetic `val_bpb` values keyed by `(algo_id, seed)`, and tags every `BpbRow` with `RunBackend::Mocked`. There is no GPU here, no Python spawn, no `val_bpb < 1.07063` claim.
2. **No I/O at this tier.** Bronze-tier deps only (`serde`, `serde_json`, `uuid`, `hex`, `sha2`, `thiserror`, plus the two SR-ALG sibling rings). The concrete tonic-gRPC trainer-runner backend lands in BR-IO once SR-02 is in.

## Public API

```rust
let arena = AlgorithmArena::with_mock();
let id = arena.register(spec, &entry_bytes)?;   // hash-verifies before storing
let row = arena.run_one(id, FIBONACCI_SEEDS[0])?;
arena.list();                                   // (AlgorithmId, name) sorted by name
```

`register` always hash-verifies the supplied `entry_bytes` against `spec.entry_hash` before storing. Mismatch → `ArenaError::EntryHashMismatch { expected, actual }` with both hex hashes for triage.

## Constitutional compliance

| Rule | Honored |
|---|---|
| **R-RING-FACADE-001** | outer `src/lib.rs` re-exports only |
| **R-RING-DEP-002** | Bronze-tier deps; no tokio, no subprocess, no HTTP |
| **R-RING-BR-004** | Bronze ring exposed via the GOLD II crate facade |
| **R-L6-PURE-007** | hashes `entry_path` bytes, never spawns Python |
| **L13** | I-SCOPE: only `rings/BR-OUTPUT/` |
| **I5** | README.md, TASK.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs |

## Tests — 10/10 GREEN

```
register_accepts_matching_hash
register_rejects_hash_mismatch
list_is_stable_sorted_by_name
run_one_returns_bpb_row_for_registered_algo
run_one_unknown_algo_errors
mocked_backend_is_deterministic_per_seed
mocked_backend_varies_with_seed
integration_e2e_ttt_register_then_run_three_seeds
entry_hash_mismatch_reports_both_hashes
run_one_propagates_backend_tag
arena_get_returns_clone
```

`cargo clippy -p trios-algorithm-arena-br-output --all-targets -- -D warnings` → **0 warnings**.

## Follow-up (out of scope for this PR)

- **SR-02 lands** → add `PythonTrainer` impl in BR-IO ring; tag rows `RunBackend::Real`.
- **bpb_samples mirror** → BR-IO ring writes captured `BpbRow` to Neon with the Coq theorem id.
- **GPU sweep PR** → registers `e2e-ttt` spec, runs F₁₇/F₁₈/F₁₉, checks `evaluate_gate`.
