# SR-03 — BPB Writer (BPB + EMA + Neon write path)

**Soul-name:** `Bit Bookkeeper` · **Codename:** `LEAD` · **Tier:** 🥈 Silver

> Closes #451 · Part of #446 · Anchor: `φ² + φ⁻² = 3`
> INV-8 PROVEN: `α_φ = φ⁻³ / 2 ≈ 0.1180` (Theorem 3.1 SAC-1)

## What this ring does

The canonical BPB write path. Composes:

- `EmaPhiBand` — exponential moving average with the φ-band-locked decay `α_φ = φ⁻³/2`.
- `BpbSink` trait — pluggable persistence boundary (mock in unit tests, sqlx/tokio-postgres adapter in BR-IO).
- `BpbWriter` — bounds one writer to one [`Scarab`], stamps every row with the EMA, forwards to the sink.

Every BPB observation in the IGLA race fleet flows through this ring. SR-04 gardener and BR-OUTPUT both read what SR-03 writes.

## INV-8 — α_φ = φ⁻³ / 2

The Coq.Reals theorem in [trios#330](https://github.com/gHashTag/trios/issues/330) pins the EMA decay. The constructor refuses any α outside `[0.10, 0.13]` and surfaces drift as `WriteErr::PhiBandOutOfRange`. Two boundary tests (`inv8_lower_bound_inclusive`, `inv8_upper_bound_inclusive`) lock the band.

## API

```rust
pub const PHI_BAND_ALPHA: f64;        // 0.118033988749895…
pub const PHI_BAND_LOW: f64;          // 0.10
pub const PHI_BAND_HIGH: f64;         // 0.13

pub struct EmaPhiBand { … }
impl EmaPhiBand {
    pub fn new() -> Self;                                 // canonical α_φ
    pub fn with_alpha(α: f64) -> Result<Self, WriteErr>;  // tests only
    pub fn update(&mut self, x: f64) -> f64;
    pub fn alpha(&self) -> f64;
    pub fn state(&self) -> Option<f64>;
}

pub trait BpbSink {
    fn put(&mut self, row: &BpbSampleRow)
        -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
}

pub struct BpbWriter { … }
impl BpbWriter {
    pub fn for_scarab(s: &Scarab) -> Self;
    pub fn for_job(id: JobId) -> Self;
    pub async fn write_one<S: BpbSink + ?Sized>(
        &mut self,
        sink: &mut S,
        row: &BpbSampleRow,
    ) -> Result<BpbSampleRow, WriteErr>;
}

pub enum WriteErr { PhiBandOutOfRange { alpha }, Sink(String), JobIdMismatch { … } }

pub const SCHEMA_SQL: &str;          // idempotent CREATE TABLE IF NOT EXISTS
```

## Tests (12/12 GREEN)

| Test | Asserts |
|---|---|
| `phi_band_alpha_correct` | `α_φ ≈ 0.1180` and lies in `[0.10, 0.13]` |
| `ema_first_update_initialises` | first call seeds EMA with the value |
| `ema_update_converges` | constant input ⇒ EMA → input within `1e-6` |
| `inv8_violation_triggers_error` | α=0.5 ⇒ `PhiBandOutOfRange` |
| `inv8_lower_bound_inclusive` | `0.10` accepted, `0.10 - ε` rejected |
| `inv8_upper_bound_inclusive` | `0.13` accepted, `0.13 + ε` rejected |
| `write_one_mock_success` | first row stamped with `ema = bpb` |
| `write_one_o1_latency` | 1000 mock writes < 100 ms (≈ O(1)) |
| `write_one_propagates_sink_error` | `Sink` error path |
| `write_one_rejects_mismatched_job_id` | wrong `job_id` ⇒ `JobIdMismatch`, sink untouched |
| `bpb_row_serde_roundtrip` | re-asserts SR-00 wire format |
| `schema_sql_idempotent_keywords_present` | every `CREATE TABLE/INDEX` uses `IF NOT EXISTS` |
| `schema_sql_mentions_required_tables` | `scarabs`, `bpb_samples`, `heartbeats` all present |

## Why a sink trait, not a hard sqlx dep

SR-03 stays Silver-tier with **no I/O**. The concrete `tokio-postgres` adapter ships in a separate BR-IO ring so SR-04 gardener and BR-OUTPUT can mock the sink in unit tests. `R-RING-DEP-002` keeps SR-03's deps to `sr-00 + serde + serde_json + chrono + thiserror`.

## Backward compatibility

After merge, the legacy `crates/trios-igla-race/src/bpb.rs` and `crates/trios-igla-race/src/ema.rs` can `pub use trios_igla_race_pipeline_sr_03::*;` to keep the live fleet building. Optional follow-up PR — not required for SR-03 to land.

## Build & test

```bash
cargo build  -p trios-igla-race-pipeline-sr-03
cargo clippy -p trios-igla-race-pipeline-sr-03 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-03
```

## Laws

- L1 ✓ no `.sh`
- L3 ✓ clippy clean
- L6 ✓ no synchronous I/O — async via the trait, no global runtime
- L13 ✓ I-SCOPE: this ring only
- L14 ✓ `Agent: Bit-Bookkeeper` trailer
- R-RING-DEP-002 ✓ no `sqlx`, no `tokio-postgres`, no `reqwest` in this ring
