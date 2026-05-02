# BR-OUTPUT TASK

Closes #460 · Part of #446 · Soul: `Arena-Anchor`

## Acceptance

- [x] `AlgorithmArena::register(spec, entry_bytes)` — hash-verify before store
- [x] `AlgorithmArena::run_one(algo, seed)` — returns `BpbRow` via `TrainerBackend`
- [x] `AlgorithmArena::list()` — `(AlgorithmId, name)` sorted by name
- [x] `AlgorithmArena::get(id)` — fetch registered spec
- [x] `MockedTrainer` deterministic backend (synthetic `val_bpb` keyed by `(algo_id, seed)`)
- [x] `RunBackend::{Real, Mocked}` provenance tag on every row
- [x] `ArenaError::{EntryHashMismatch, UnknownAlgorithm, SeedRejected}`
- [x] Integration test: register e2e-ttt, run three Fibonacci seeds → 3 distinct `BpbRow` ids
- [x] R-L6-PURE-007: entry_path verified, never executed
- [x] R-RING-DEP-002: Bronze-tier deps only

## Out of scope (R5-honest)

- Real `TrainerBackend` over SR-02 `PythonRunner` (lives in BR-IO; SR-02 is #454)
- Neon `bpb_samples` mirror (lives in BR-IO)
- GPU `val_bpb < 1.07063` claim (lives in a follow-up GPU sweep PR through SR-02)
