P0 Audit - Baseline validation
# P0 Audit - Baseline Validation

## Purpose
Validate that trios-trainer can reproduce champion baseline:
- Config: champion.toml loads correctly
- INV-8: LR is within φ-band [0.001, 0.01]
- Model: Correct architecture (d_model=384, n_layers=4)
- Training: Run to BPB ≈ 2.2393 at 27K steps, seed=43

## Exit Criteria (R5-Honest)
- ✅ test_champion_config_loads() passes
- ✅ test_inv8_lr_validation() passes
- ⏸ reproduce_champion_full() - requires full 27K-step run (manual)

## Files
- tests/champion_reproduction.rs - basic config validation
- assertions/champion_lock.txt - expected hash (to be added after full run)

## Owner
@trios-trainer team

## Timeline
- Created: 2026-04-27
- Status: Phase 0.1 - Infrastructure ready
