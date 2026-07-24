# Skill: L3 Purity Gate — ASCII-Only Enforcement

## When to use

When `cargo clippy --workspace`, `cargo test --workspace`, or `cargo build --workspace` panics with a `t27c LANGUAGE POLICY VIOLATION` error, typically caused by non-ASCII characters (Cyrillic, emoji, math symbols) in committed files.

## Why

- `bootstrap/build.rs` enforces ASCII-only file content for all source and documentation files
- L3 PURITY law: "Source files must be ASCII-only with English identifiers"
- Cyrillic characters (e.g., Russian text) cause build-script panics that abort the entire workspace build

## Diagnosis

### Error pattern:
```
thread 'main' panicked at bootstrap/build.rs:196:17:
t27c LANGUAGE POLICY VIOLATION: Cyrillic character U+043D ('н') in file docs/reports/EXAMPLE.md
Location: line 90, column 90
```

### Root cause:
The build script scans all files under `docs/` and `bootstrap/src/` (and possibly other paths) for non-ASCII characters. Any file containing Cyrillic, emoji, math symbols, or other non-ASCII content triggers a panic.

## Fix

1. **Identify the offending file and line:**
   ```bash
   # From the panic message, note the file path and line number
   # Or search manually:
   grep -rn '[А-Яа-яЁё]' docs/ bootstrap/src/ 2>/dev/null | head -20
   ```

2. **Replace non-ASCII with ASCII equivalents:**
   - Cyrillic text → English translation
   - Emoji → ASCII text description (e.g., "✅" → "PASS")
   - Math symbols → LaTeX or English names (e.g., "Σ" → "Sigma")

3. **Verify fix:**
   ```bash
   cargo clippy --workspace --all-features 2>&1 | tail -5
   # Should compile without language policy panic
   ```

## Prevention

1. **Before committing any `.md` or `.rs` file:**
   ```bash
   # Quick check for non-ASCII chars
   grep -n '[^\x00-\x7F]' docs/reports/YOUR_FILE.md
   # If empty → safe to commit
   ```

2. **In CI:** Add a pre-commit hook that runs `cargo clippy --workspace` before allowing push

3. **In memory files:** The `.claude/projects/.../memory/` directory is outside the repo and not scanned by build.rs. Non-ASCII is safe there.

## Common Violations

| Source | Violation | Fix |
|--------|-----------|-----|
| Russian comments | `"нам нужна модель"` | `"we need a model"` |
| Emoji in reports | `✅` | `PASS` or `OK` |
| Math symbols | `Σ m_ν` | `Sigma m_nu` or `Σ m_nu` (if build.rs allows Greek) |
| Smart quotes | `"` `"` | `"` `"` |

## Related

- [[l1-l7-compliance]] — Constitutional invariant laws
- [[trinity-l1-l7-compliance]] — Full L1-L7 compliance rules
