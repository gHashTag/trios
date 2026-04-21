# Binary Discovery — L7 Lesson

## Observation
Multiple hours lost referring to fictional binaries:
- train_real_fixed_v2 (E0061 compile error)
- train_phase_a_v2 (non-existent)

Correct binary discovered only via `ls src/bin/ examples/`:
- examples/phase_b_fine.rs ✅

## Rule
**BEFORE any `cargo run --bin X` or `cargo run --example X`:**

1. `ls src/bin/ examples/ tests/` — verify path exists
2. `cargo run --bin X -- --help` OR read source — verify flags exist
3. `cargo build --bin X` — verify compiles before long runs

**Never trust agent memory of binary names. Always verify with filesystem.**

## Impact
- Saved 7+ hours chasing fictional binaries
- Phase B training continued via correct path
- Lesson: "trust but verify" principle applied
