# tri-parser-array-literal

## When to use

When t27 specs using `[elem1, elem2, ...]` array-literal syntax generate empty array bodies in any backend (C, Zig, Verilog, Rust), or when the parser misparses inline array elements.

## Steps

1. Locate `parse_array_literal` in `bootstrap/src/compiler.rs` (around line 2649).
2. Observe that it currently assumes all `[...]` expressions are followed by `{...}`.
3. Identify that the bracket-content collector greedily consumes everything until `]` into `bracket_content`, treating it as a size/type string.
4. **Fix strategy (save/restore two-pass):**
   - Add `#[derive(Clone)]` to `Lexer` and `Parser` structs.
   - After `LBracket`, **clone the parser state** (`let saved = self.clone();`).
   - **Attempt element list:** call `parse_expr()` repeatedly, advancing through commas until `RBracket`.
   - After `RBracket`, check the next token: if it is `Ident` or `LBrace`, this was actually a type annotation (e.g. `[3]f64{...}`). Set `is_elem_list = false`.
   - If `is_elem_list` is true, return the node with populated `children`.
   - Otherwise, **restore state** (`*self = saved;`) and fall through to the original type-annotation / repeat parsing logic.
5. Rebuild bootstrap (`cd bootstrap && cargo build --release`).
6. Update `bootstrap/stage0/FROZEN_HASH`.
7. Regenerate affected seals (expect 30–35 `gen_hash_c` mismatches).
8. Verify with `t27c suite --repo-root .`.

## Pitfalls

- Do NOT break existing `[_]Type{...}` or `[N]Type{...}` syntax used in specs like `state_machine_mapping.t27`.
- **Save/restore clones the entire Lexer source bytes** — acceptable for current spec sizes but may need optimisation for very large files.
- Empty-array syntax `[]` or `[ ]` needs explicit handling to avoid infinite loops.
- Nested array literals (`[[1, 2], [3, 4]]`) require recursive `parse_expr()` calls that may interact with save/restore; test thoroughly.
