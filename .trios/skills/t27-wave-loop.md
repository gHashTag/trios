# t27 Wave Loop — Spec-First Hardware-Lowering Skill

## Description

Autonomous FPGA-loop execution pattern for the t27 (Trinity S³AI) spec-first
tri-27 compiler. Each wave picks one cooperation variant, writes/updates `.t27`
specs, fixes the Rust Verilog/Zig/C/Rust backend, reseals affected specs, adds
adversarial scratch witnesses, runs `./scripts/tri test --icarus-lowerable
--fast`, `./scripts/tri verify --lean-lowerable`, and `lake build
Trinity.IcarusLowerable.Soundness`, then closes out with a report and three
variants for the next wave.

## How to Use

1. Read `.trinity/current-issue.md` to get the wave number, issue, selected
   variant, and residual boundaries.
2. Read the matching `docs/reports/FPGA_LOOP_COOPERATION_WNNN_*.md` for the
   three proposed variants.
3. Create a decomposed plan in `.claude/plans/wave-loop-NNN.md`.
4. Implement the selected variant in `bootstrap/src/compiler.rs` (and/or
   `proofs/lean4/Trinity/IcarusLowerable/Lemmas.lean` / `Soundness.lean`).
5. Add scratch witnesses under `specs/scratch/wNNN_*.t27`.
6. Add a Rust integration test in `bootstrap/tests/wNNN_*.rs` for generated
   Verilog shape checks.
7. Run gates:
   - `cd bootstrap && cargo build --release`
   - `cargo test -p t27c --bin t27c`
   - `cargo test -p t27c --tests`
   - `./scripts/tri test --icarus-lowerable --fast`
   - `./scripts/tri verify --lean-lowerable`
   - `cd proofs/lean4 && lake build Trinity.IcarusLowerable.Soundness`
8. Reseal affected specs with `./target/release/t27c seal --save <path>`.
9. Write `docs/reports/WAVE_LOOP_NNN_CLOSEOUT.md` and
   `docs/reports/FPGA_LOOP_COOPERATION_WNNN+1_*.md`.
10. Update `.trinity/current-issue.md` to the next wave.
11. Append key learnings to `.trinity/experience.md`.

## Pattern Library

### aos-flattened-literal-count
Multi-dimensional struct-literal arrays such as `[2][3]Pt{...}` are parsed with
`extra_size=2` and `children.len() == 2 * 3` (flat rows). Consumers must compute
the expected count as `outer_size * inner_product`, not `size == children.len()`.

**Reference:** `bootstrap/src/compiler.rs`, `try_emit_array_of_struct_literal_packed`

### aos-return-packed-tmp
A function returning an array-of-structs must be materialized as a packed
Verilog temporary (`reg [W-1:0] _aos_ret_tmp_N`) before slicing. Both Icarus and
Yosys reject slicing a function-call result directly, and index chains like
`call()[i].field[j]` require a single `[high:low]` slice on that temporary.

**Reference:** `bootstrap/src/compiler.rs`, `try_emit_array_of_struct_call_field` /
`try_emit_array_of_struct_call_field_index`

### module-function-aos-cross-boundary
Module-level and function-boundary 2-D scalar-struct arrays lower as a single
packed Verilog vector. Keep the shared AST unchanged by parsing the array-literal
text on demand in the Verilog backend. Track module-level and parameter array
types in separate maps so the same linearized slice expression works for locals,
module symbols, and function inputs.

**Reference:** `bootstrap/src/compiler.rs`, `parse_array_literal_text`,
`module_types`, `param_types`, `emit_packed_array_literal_concat`

### module-aos-assignment-initial-block
Module-level whole-array assignment from an AOS-returning call (`g = make_grid();`)
must not live in the shared `always @(*)` block. Pre-declare a module-scope packed
temporary and emit the assignment + unpacking in an `initial begin` block to avoid
a time-0 race with Icarus `initial` test blocks.

**Reference:** `bootstrap/src/compiler.rs`, `module_aos_assign_call_temps` /
`module_stmt_aos_call_assign_info`

### signed-packed-slice
A part-select of a signed packed vector is unsigned in Verilog. Element reads
from signed packed primitive arrays (`[N]i8`) must wrap the slice with
`$signed(...)` so comparisons, arithmetic, and typed VCD probes preserve t27's
two's-complement semantics.

**Reference:** `bootstrap/src/compiler.rs`, `try_emit_primitive_array_access`

### cocotb-test-block-locals
The Python reference model must know the full declared type of test-block local
variables (including array dimensions) to infer the correct width/signedness for
VCD cross-checks. Collect `StmtLocal` nodes before processing assertions and
bind their packed values temporarily while evaluating the block.

**Reference:** `scripts/cocotb_ref_model.py`, `_collect_assertions`, `_resolve_full_type`

### reseal-after-verilog-layout-change
Any backend change that alters generated Verilog must be followed by
`./target/release/t27c seal --save <path>` for every affected spec. The suite
fails on mismatched `gen_hash_verilog` even when behavior is unchanged.

**Reference:** `.trinity/seals/`, `./scripts/tri test`

### scratch-is-not-completeness
`./scripts/tri verify --lean-lowerable` skips `specs/scratch/` when generating
`Completeness.lean`. If a new lowerable shape must be exercised by the Lean
completeness gate, add a minimal non-scratch corpus spec under `specs/igla/`.

**Reference:** `specs/igla/w521_2d_aos_param_soundness.t27`,
`proofs/lean4/Trinity/IcarusLowerable/Completeness.lean`

## Literature Index

| Topic | Reference |
|-------|-----------|
| Packed arrays / structs in Verilog | IEEE Std 1800-2017, §7.2.1 / §7.6 |
| Synthesizable multi-dimensional arrays | Sutherland & Mills, *“Can My Synthesis Compiler Do That?”* (SNUG 2013) |
| Source-to-Verilog equivalence proofs | Herklotz et al., *“Formal Verification of High-Level Synthesis”* (OOPSLA 2021) |
| Modular hardware verification | Choi et al., *“Kami”* (ICFP 2017) |

*φ² + φ⁻² = 3 | TRINITY*
