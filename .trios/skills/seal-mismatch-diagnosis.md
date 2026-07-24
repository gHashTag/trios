# Skill: Seal Mismatch Diagnosis

## When to use

When `t27c suite --repo-root .` reports `Seal mismatches: N` (where N > 0), especially after compiler changes or spec modifications.

## Why

- Seal mismatches indicate generated code hash ≠ expected hash stored in `.t27` seal files
- They are often caused by stale seals after compiler fixes, not by actual bugs
- False positives waste time if you treat every mismatch as a bug

## Diagnosis Steps

1. **Confirm reproducibility:**
   ```bash
   ./target/release/t27c suite --repo-root . 2>&1 | grep -E "Seal|FAIL"
   # Run TWICE. If mismatches disappear on second run → stale cache.
   ```

2. **Identify affected specs:**
   ```bash
   ./target/release/t27c suite --repo-root . 2>&1 | grep "SEAL MISMATCH"
   # Or look for spec names in verbose output
   ```

3. **Check if seals need regeneration:**
   ```bash
   git status --short | grep "\.t27$"  # Any modified specs?
   git log --oneline -3                # Any recent compiler changes?
   ```

4. **Categorize the mismatch:**
   | Pattern | Likely Cause | Action |
   |---------|--------------|--------|
   | Mismatches after compiler fix | Expected — codegen changed | Regenerate seals via `tri seal` |
   | Mismatches with no code changes | Stale cache / race condition | Re-run suite |
   | Mismatches in single spec | Spec was hand-edited | Check for manual changes |
   | Mismatches across many specs | Compiler bug or global change | Root-cause before regenerating |

## Fix: Seal Regeneration Protocol

```bash
# 1. Verify no actual bugs (parse/typecheck/gen all pass):
./target/release/t27c suite --repo-root . 2>&1 | grep -E "Parse|Typecheck|Gen .* failures"

# 2. Regenerate seals for affected specs:
cd /Users/playra/t27
for spec in $(grep -l "seal mismatch" /tmp/suite_output.txt 2>/dev/null || echo ""); do
    ./target/release/t27c seal --repo-root . "$spec"
done

# 3. Or regenerate all:
find specs -name "*.t27" -exec ./target/release/t27c seal --repo-root . {} \;

# 4. Verify:
./target/release/t27c suite --repo-root . 2>&1 | tail -5
# Expected: "Seal mismatches: 0"
```

## Anti-patterns

- **Don't ignore mismatches blindly** — confirm they are expected (post-compiler-fix) before regenerating
- **Don't regenerate during active debugging** — if you're fixing a compiler bug, seals will keep changing
- **Don't commit unregenerated seals** — always regenerate and commit updated seals in the same PR

## Related

- [[trinity-bootstrap-seal]] — FROZEN_HASH rules and seal ceremony
- [[tri-pipeline]] — Running `tri seal` and `tri verify`
