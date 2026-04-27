# P0 Audit: Champion Reproduction (Seed 43)

> **Status**: In Progress
> **Date**: 2026-04-27
> **Owner**: repro-auditor
> **Issue**: P0 Audit from TRAINING_FLOW_V2.md

## Hypothesis

`configs/champion.toml --seed 43` reproduces `BPB = 2.2393 +/- 0.01 @ step 27000` on a fresh machine.

## Target Baseline

| Metric | Value | Reference |
|--------|-------|-----------|
| BPB | 2.2393 | gHashTag/trios@2446855 |
| Step | 27000 | - |
| Seed | 43 | - |
| Tolerance | +/- 0.01 | Exit criterion |

## Architecture

```toml
[model]
d_model = 256
n_layers = 2
n_heads = 4
vocab_size = 32000
```

## Invariants

- **INV-1**: LR in φ-band [0.002, 0.007]
- **INV-8**: LR in [1e-3, 1e-2]

## Test Execution

Run with:
```bash
cargo test --release reproduce_champion -- --ignored
```

## Results

*Pending execution*

## Ledger Row

Expected R7 format:
```
BPB=<v> @ step=<N> seed=43 sha=<HEAD7> jsonl_row=<L> gate_status=below_target_evidence
```

## Exit Criterion

- [ ] Ledger emits `BPB=2.2393 +/- 0.01 @ step=27000 seed=43`
- [ ] Row passes R8 (step >= 4000)
- [ ] Row passes R9 (embargo check)
- [ ] `assertions/champion_lock.txt` updated with champion@<sha>

## Falsification

BPB drift > 0.05 → bisect against `gHashTag/trios@2446855` before any other phase.

---

**Anchor**: `phi^2 + phi^-2 = 3`
