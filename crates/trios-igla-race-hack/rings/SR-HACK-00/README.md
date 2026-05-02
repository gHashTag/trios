# SR-HACK-00: Glossary

Glossary of Trinity IGLA Race ecosystem terminology.

## Exports

- `Term` - Unified glossary term type
- `PipelineO1` - End-to-end test-time training pipeline
- `AlgorithmEntry` - Algorithm entry point with metadata
- `Lane` - Execution lane enum
- `Gate` - Falsifier checkpoint
- `RingTier` - Ring hierarchy

## Invariants

- **I1:** No external dependencies beyond serde, chrono, uuid
- **I2:** All types are serde-serializable
- **I3:** No business logic in this ring

See [RING.md](RING.md) for detailed specification.
