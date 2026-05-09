# RING — BR-IO-CHAT-07 (async wire-emitter)

**Tier**: Bronze (I/O) · **Lane**: L-CHAT-7-async · **Wave**: 7

## Purpose

Drive `CR-CHAT-07::CoverScheduler` over a real `tokio` clock so the
mesh node can emit `Real` and `Cover` envelopes onto the wire on a
fixed cadence. This is the I/O twin of the pure Silver ring
`CR-CHAT-07`.

## Rules

- **No business logic.** All scheduling decisions delegate to the
  wrapped `CoverScheduler`. This ring only owns: the timer, the
  channel, and graceful-shutdown.
- **Deterministic under `tokio::time::pause`.** All 5 unit tests run
  under `#[tokio::test(start_paused = true)]` so the emission stream
  is byte-reproducible.
- **No randomness.** Cover content is the caller's job (random
  ciphertext at the right padding class — see CR-CHAT-04). This ring
  only signals *when*.

## Surface

- `WireEmitter::new(tick, sender)` — custom cadence
- `WireEmitter::with_default_tick(sender)` — 1-second cadence
  (smallest canonical bin from `CR-CHAT-07::CANONICAL_GAPS_MS`)
- `WireEmitter::enqueue_real()` — buffer one real envelope
- `WireEmitter::run_for(n_ticks).await` — emit exactly `n_ticks`
- `WireEmitter::queue_depth()` / `ticks()` — observability

## Invariants

- **R-CHAT-10 (iii)** — exactly one `Emission` per tick.
- **R-CHAT-10 (iv)** — async stream equals pure-scheduler stream
  given identical enqueue pattern.
- **R-CHAT-10 (v)** — silence is forbidden; empty queue ⇒ `Cover`.

## Tests

| ID | Property | Status |
|----|----------|--------|
| AE-01 | empty queue ⇒ Cover every tick | `[VERIFIED]` |
| AE-02 | queued reals drain first, then Cover | `[VERIFIED]` |
| AE-03 | async stream ≡ pure scheduler | `[VERIFIED]` |
| AE-04 | closed channel halts gracefully | `[VERIFIED]` |
| AE-05 | logical time advances by `n*tick` | `[VERIFIED]` |
| G-C7-async | green summary | `[VERIFIED]` |

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · UNLINKABLE`
