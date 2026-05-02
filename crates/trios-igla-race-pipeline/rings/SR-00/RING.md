# SR-00: Scarab Types

**Purpose:** Core type definitions for IGLA Race pipeline extracted from monolith.

**Scope:** Types and minimal logic only. No business logic.

**Dependencies:** serde, serde_json, chrono, uuid (dep-free beyond std).

**Exports:** `TrialId`, `TrialConfig`, `JobStatus`, `TrialRecord`, `RungResult`, `ExperienceEntry`.

---

## Type Definitions

| Type | Source | Description |
|------|--------|-------------|
| `TrialId` | extracted from neon.rs | UUID-based trial identifier |
| `TrialConfig` | extracted from neon.rs | Hyperparameters (arch, d_model, lr, wd, n_gram, etc.) |
| `JobStatus` | synthesized | Trial lifecycle states (Running, Pruned, Complete, IGLAFound, Error) |
| `TrialRecord` | synthesized | Complete trial state with rung results |
| `RungResult` | synthesized | ASHA rung measurement |
| `ExperienceEntry` | extracted from neon.rs | Failure memory protocol entry |

## Invariants

- I1: No external dependencies beyond serde, chrono, uuid
- I2: All types are serde-serializable
- I3: No business logic in this ring
