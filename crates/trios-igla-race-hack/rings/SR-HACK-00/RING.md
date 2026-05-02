# SR-HACK-00: Glossary

**Purpose:** Define shared terminology for Trinity IGLA Race ecosystem.

**Scope:** Types and constants only. No business logic.

**Dependencies:** None (dep-free).

**Exports:** `Term`, `PipelineO1`, `AlgorithmEntry`, `Lane`, `Gate`, `RingTier`.

---

## Glossary Terms

| Term | Description |
|------|-------------|
| `PipelineO1` | End-to-end test-time training pipeline with O(1) complexity per chunk |
| `AlgorithmEntry` | Entry point for algorithm spec (train_gpt.py path, hash, env vars) |
| `Lane` | Execution lane (scarab, strategy-queue, trainer-runner, bpb-writer, gardener, railway-deployer) |
| `Gate` | L-f3 / L-f4 falsifier checkpoint |
| `RingTier` | Ring hierarchy (SR, MR, LR) |

## Invariants

- I1: No external dependencies beyond serde, chrono, uuid
- I2: All types are serde-serializable
- I3: No business logic in this ring
