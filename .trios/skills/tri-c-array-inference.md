# tri-c-array-inference

## When to use

When the C backend in `bootstrap/src/compiler.rs` miscompiles array literals or array-initialized local variables, especially when t27 typed literals (`0i64`, `1.0f64`) are involved.

## Steps

1. Locate `gen_c_stmt` in `bootstrap/src/compiler.rs`.
2. Find the `NodeKind::StmtLocal` fallback path.
3. If `extra_type` is empty and RHS is `ExprArrayLiteral`, infer element type via helper `infer_array_elem_type(node)`.
4. **Add `strip_type_suffix(value: &str) -> String`** to strip numeric suffixes (`i64`, `u64`, `f64`, `usize`, etc.) before parsing literal values for type inference.
5. Integrate `strip_type_suffix` into `infer_array_elem_type` before `parse::<i64>()` and `parse::<f64>()`.
6. Construct C type with `Self::type_to_c(&format!("{}[]", typ))`.
7. Refactor `gen_c_expr` `ExprArrayLiteral` to reuse the same helper.
8. Rebuild bootstrap (`cd bootstrap && cargo build --release`).
9. Update `bootstrap/stage0/FROZEN_HASH` with `sha256sum bootstrap/src/compiler.rs`.
10. Run `./scripts/tri seal --save` on any specs with seal mismatches.
11. Verify with `t27c suite --repo-root .`.

## Pitfalls

- **Typed literal suffixes break inference:** `"0i64".parse::<i64>()` returns `Err`; always strip suffixes first.
- Parser body truncation on tiny synthetic specs prevents direct C-output inspection; verify on real specs from `specs/` tree instead.
- C backend changes always cascade to `gen_hash_c` seals; expect 5–15 seal regenerations after array-related compiler.rs edits.
