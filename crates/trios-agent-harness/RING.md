# RING — trios-agent-harness (🥉 Silver Crate)

| Field | Value |
|-------|-------|
| Metal | 🥉 Silver |
| Role  | Agent harness — ACP runtime, catalog, message queue, turn registry |
| State | Scaffold (Wave 0) |

## Ring Structure (L-ARCH-001)
```
crates/trios-agent-harness/
├── src/lib.rs      ← re-export facade
└── rings/
    └── AH-00/   ← core types (scaffold)
```
More rings are added as TS logic is ported (see RUST_CONSOLIDATION_PLAN.md).
