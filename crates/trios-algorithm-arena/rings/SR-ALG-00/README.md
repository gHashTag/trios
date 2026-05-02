# SR-ALG-00 — Arena Types (algorithm-spec metadata)

**Soul-name:** `Arena Architect` · **Codename:** `LEAD` · **Tier:** 🥉 Bronze · **Kingdom:** Rust

> Closes #450 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## What this ring does

Dependency-free typed metadata for one algorithm submitted to the
TRIOS IGLA arena. SR-ALG-00 is the **bottom of the GOLD II graph** —
SR-ALG-01..03 + BR-OUTPUT all import their wire format from here.

## Public types

| Type | Wire-format role |
|---|---|
| `AlgorithmId` | UUID v4 newtype |
| `EntryHash([u8; 32])` | SHA-256 of the entry script (lowercase hex JSON) |
| `EnvVar(String)`, `EnvValue(String)` | env-pair newtypes |
| `GoldenState([u8; 32])` | SHA-256 of the convergence checkpoint (optional) |
| `AlgorithmSpec` | full manifest (id, name, entry_path, entry_hash, env, golden_state_hash?, theorem?) |
| `AlgorithmSpec::new(name, path, hash)` | cheap constructor |
| `AlgorithmSpec::verify_hash(actual)` | bool |

## Critical: no Python in `crates/`

`AlgorithmSpec::entry_path` MUST point *outside* `crates/` (typically
into `parameter-golf/records/...`). Real Python spawn is the job of
SR-02 trainer-runner — never this crate. R-L6-PURE-007.

## Dependencies

- `serde` (derive)
- `uuid` (`v4`, `serde`)
- `hex` — lowercase hex serialisation for 32-byte hashes
- `serde_json` (dev only)

No `tokio`, no `sqlx`, no `reqwest`. R-RING-DEP-002.

## Tests (12/12 GREEN)

| Test | Asserts |
|---|---|
| `algorithm_id_unique_per_instance` | newtype freshness |
| `entry_hash_32_bytes_serialises_as_hex64` | 32 bytes → 64 hex chars |
| `entry_hash_rejects_wrong_length` | 31-byte hex string fails to deserialise |
| `golden_state_serialises_as_hex64` | round-trip |
| `env_empty_vec_valid` | empty env compiles + roundtrips |
| `entry_path_from_str_round_trips` | `PathBuf` parity |
| `theorem_optional` | absent field omitted from JSON |
| `golden_state_hash_optional` | absent field omitted from JSON |
| `verify_hash_correct` | match path |
| `verify_hash_wrong_fails` | mismatch path |
| `serde_roundtrip_full_spec` | every field roundtrips |
| `schema_field_names_stable` | JSON keys frozen for downstream rings |

## Build & test

```bash
cargo build  -p trios-algorithm-arena-sr-alg-00
cargo clippy -p trios-algorithm-arena-sr-alg-00 --all-targets -- -D warnings
cargo test   -p trios-algorithm-arena-sr-alg-00
```

## Laws

- L1 ✓ no `.sh`
- L3 ✓ clippy clean
- L6 ✓ no I/O, no async
- L13 ✓ I-SCOPE: this ring only
- L14 ✓ `Agent: Arena-Architect` trailer
- R-RING-DEP-002 ✓ deps = `serde + uuid + hex` (+ `serde_json` dev)
- R-L6-PURE-007 ✓ no `.py` in this crate; entry_path is a *reference*
