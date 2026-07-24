# RING — trios-openclaw (🥉 Silver Crate)

| Field | Value |
|-------|-------|
| Metal | 🥉 Silver |
| Role  | OpenClaw — container/VM orchestration + agent gateway |
| State | Scaffold (Wave 0) |

## Ring Structure (L-ARCH-001)
```
crates/trios-openclaw/
├── src/lib.rs      ← re-export facade
└── rings/
    └── OC-00/   ← core types (scaffold)
```
More rings are added as TS logic is ported (see RUST_CONSOLIDATION_PLAN.md).
