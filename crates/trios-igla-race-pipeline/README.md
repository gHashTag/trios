# SR-00: Scarab Types

Core type definitions for IGLA Race pipeline extracted from monolith.

## Exports

- `TrialId` - UUID-based trial identifier
- `TrialConfig` - Hyperparameter configuration (arch, d_model, lr, wd, n_gram, etc.)
- `JobStatus` - Trial lifecycle states (Running, Pruned, Complete, IGLAFound, Error)
- `TrialRecord` - Complete trial state with rung results
- `RungResult` - ASHA rung measurement
- `ExperienceEntry` - Failure memory protocol entry

## Invariants

- I1: No external dependencies beyond serde, chrono, uuid
- I2: All types are serde-serializable
- I3: No business logic in this ring
