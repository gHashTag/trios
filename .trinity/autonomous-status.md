# IGLA RACE Autonomous Status
# Last updated: 2026-04-27 10:10 UTC

## Current Progress

### trios-trainer-igla (IGLA RACE repo)
- **P0 (Audit)**: Configs and tests ready
- **P1 (Optimizer Lab)**: Deployment script created (scripts/p1-deploy.sh)
  - 3 configs: p1-adamw, p1-muon, p1-muon-cwd
  - INV-8 compliant (LR in [1e-3, 1e-2])
  - nixpacks.toml updated for config mode
  - Next: Deploy to Railway

### trios (main repo)
- **Experiments E36-E40**: Completed @ 20K steps
  - Best BPB: E39=2.5172 (Balanced config)
  - Gate-2 target (≤2.03): FAILED
  - Note: T-JEPA d_model=64 not sufficient

- **Experiments E31-E35**: Completed @ 200K steps (no logs saved)
  - Need to check NEON SQL for results

## Next Steps (Priority)

1. **Deploy P1 to Railway** - Run `scripts/p1-deploy.sh`
2. **Analyze E31-E35 results** - Check NEON SQL for BPB values
3. **Prepare new experiment configs** - Different arch/hyperparams for Gate-2
4. **Continue through P2-P5** - Following TRAINING_FLOW_V2.md

## Gate-2 Status
- Target: BPB ≤ 1.85 on 3 seeds (step ≥ 4000)
- Current best: ~2.17 (from earlier runs)
- Gap: ~0.3 BPB
- Deadline: 2026-04-30 23:59 UTC

## Configs Ready for Deployment
- `configs/lab/p1-adamw.toml` - Control (AdamW, LR=0.004)
- `configs/lab/p1-muon.toml` - Muon (η2D=0.008, η1D=0.007)
- `configs/lab/p1-muon-cwd.toml` - Muon+CWD (with cautious weight decay)

All configs: 12K steps, seed 43, d_model=256, n_layers=2
