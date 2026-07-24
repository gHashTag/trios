# t27 Trinity S³AI — Complete Skill

Complete skill for working with the Trinity t27 repository. This is the main entry point for all t27-related operations.

## Overview

**Repository:** `gHashTag/t27`  
**Working Directory:** `~/t27`  
**Current Branch:** `fix/ring-018-compiler-cleanup` (or feature branch)

**Constitutional Laws (L1-L7):**
- L1 TRACEABILITY: No code merged without `Closes #N`
- L2 GENERATION: Files under `gen/` are generated; edit `.t27` spec instead
- L3 PURITY: All source files must be ASCII-only with English identifiers
- L4 TESTABILITY: Every `.t27` spec must contain `test`/`invariant`/`bench`
- L5 IDENTITY: φ² = φ + 1 on ℝ
- L6 CEILING: `FORMAT-SPEC-001.json` + `gf16.t27` are numeric SSOT
- L7 UNITY: No new `*.sh` on critical path

## Quick Commands

### CI & PR Workflow

```
/tri ci-fix <issue-number>
```
Fixes all failing CI checks:
- Updates `docs/NOW.md` with current date
- Amends commit with `Closes #N`
- Pushes with `--force-with-lease`

```
/tri pr-create <branch-name> <issue-number> "<title>"
```
Creates properly formatted PR that passes all CI checks.

### Development

```
/tri compile
```
Builds t27c bootstrap compiler. Shows build time, warnings, errors.

```
/tri gen-rust <spec-file>
```
Generates Rust code from .t27 specs.

```
/tri now-update "<work-title>"
```
Updates `docs/NOW.md` with current date and work entry.

### Full Workflow Example

```bash
# 1. Make changes
vim bootstrap/src/compiler/mod.rs

# 2. Build and test
/tri compile

# 3. Update NOW.md
/tri now-update "Ring 018 — added compile_rust stub"

# 4. Create issue on GitHub (manual)
# → Issue #498 created

# 5. Commit and create PR
git add -A
git commit -m "feat: add compile_rust stub"
/tri pr-create fix/ring-018 498 "feat: add compile_rust stub"
```

## Project Structure

```
t27/
├── specs/              # .t27 SPECIFICATIONS — source of truth
│   ├── base/          # types, ops, constants
│   ├── numeric/       # GoldenFloat GF4-GF32, TF3, phi
│   ├── compiler/      # parser, codegen, CLI
│   └── ...
├── gen/               # GENERATED backends — DO NOT EDIT
│   ├── zig/
│   ├── c/
│   └── verilog/
├── bootstrap/          # Stage-0 compiler (Rust)
│   ├── src/
│   │   ├── main.rs
│   │   ├── compiler.rs      # Flat monolithic compiler
│   │   └── compiler/        # Modular compiler (preferred)
│   └── Cargo.toml
├── docs/              # Documentation
│   └── NOW.md         # MUST be updated for every PR
└── .github/workflows/ # CI checks
```

## Common Issues

### CI Failing: Issue Gate
**Problem:** `No 'Closes #N' found in PR title/body`  
**Solution:** Use `/tri ci-fix <issue-number>` or manually add `Closes #N` to commit message.

### CI Failing: NOW Sync Gate
**Problem:** `NOW.md date is too old`  
**Solution:** Use `/tri now-update "<title>"` or manually update `Last updated` to today's date.

### CI Failing: L1 TRACEABILITY
**Problem:** `No code merged without Closes #N`  
**Solution:** Same as Issue Gate — ensure commits have `Closes #N`.

### Build Error: module conflict
**Problem:** `file for module 'compiler' found at both compiler.rs and compiler/mod.rs`  
**Solution:** The modular compiler in `compiler/` is preferred. Consider removing or renaming `compiler.rs`.

## Ring Progress

| Ring | Status | Description |
|------|--------|-------------|
| 0-8 | SEED | Base types, numeric ops, sacred physics |
| 9-11 | SEED | Compiler: parser → codegen → Zig/C/Verilog |
| 12-14 | SEED | FPGA: MAC unit, Verilog gen, bitstream |
| 15-17 | SEED | Queen + NN orchestration, AR modules |
| 18-24 | AR | CLARA AR pipeline (ternary logic, Datalog, etc.) |
| 25-31 | GEN | Gen backends for all domains |
| 32+ | ACTIVE | Cloud orchestration, deployment |

## Current Work (Ring 018)

**Goal:** Implement `.t27 → .trib` codegen pipeline

**Status:**
- ✅ Build clean (0 errors)
- ✅ Compiler modules restored
- ✅ CI checks fixed (PR #497, Closes #498)
- ⏳ `compile_rust()` stub added (needs full implementation)

**Next Steps:**
1. Implement full Rust codegen in `compiler/codegen/rust.rs`
2. Add `Compiler` struct with proper methods
3. Test with actual .t27 specs

## Quick Reference

### Git Commands
```bash
git checkout -b fix/<name>      # Create feature branch
git push origin <branch>         # Push to GitHub
git push --force-with-lease      # Force push (safe)
```

### Build Commands
```bash
cd ~/t27/bootstrap
cargo build --release -p t27c   # Build compiler
./target/release/t27c --help     # Show help
```

### File Locations
- `~/t27/` — Repository root
- `~/.claude/skills/tri.md` — This skill file
- `~/t27/docs/NOW.md` — Current work log (MUST UPDATE!)
- `~/t27/bootstrap/src/` — Compiler source

## Version History

- **2026-04-18:** Ring 018 compiler cleanup, CI fixes
- **2026-04-16:** Ring 32 cloud orchestration
- **Earlier:** Rings 0-31 complete

---

*Last updated: 2026-04-18*  
*Skill version: 1.0.0*
