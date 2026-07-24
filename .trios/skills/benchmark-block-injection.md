# Skill: Benchmark Block Injection for Hot Primitives

## When to Use

When you need to add `bench` blocks to t27 specs that lack performance benchmarks but are on latency-critical paths. This satisfies L4 (TESTABILITY) law.

## Steps

1. **Identify hot primitives** — Look for specs that are:
   - Called frequently in generated code (e.g., debounce, task scheduling, neural kernels)
   - On critical RTL/ML paths (conv2d, multi-head attention)
   - Module registry or validation functions

2. **Check existing coverage** — Run `grep "^bench " specs/...` to confirm absence.

3. **Write bench blocks** — Follow t27 syntax:
   ```t27
   bench "description" {
       // setup
       var result = function(args);
       _ = result;  // prevent optimization away
   }
   ```

4. **Run suite** — `./target/release/t27c suite --repo-root .`

5. **Regenerate seals** — For any spec with `spec_hash: MISMATCH`, run `./target/release/t27c seal --save <path>`.

6. **Verify** — Re-run suite until 0 seal mismatches.

## Known Pitfalls

- Some files (e.g., `btree.t27`, `lru_cache.t27`) use `.tri` syntax, not t27 — adding `bench` will fail parser. Migrate syntax first.
- Zig-style bench blocks (`@setEvalBranchQuota`) in files like `ternary_gates.t27` are NOT recognized by t27c parser. They are legacy artifacts.
- Always add `_ = result;` at end of bench to prevent dead-code elimination.

## Why It Matters

L4 law requires every `.t27` spec contain `test`/`invariant`/`bench`. Hot primitives without benchmarks create blind spots in performance regression detection.
