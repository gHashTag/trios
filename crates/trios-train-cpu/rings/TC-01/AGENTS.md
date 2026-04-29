# Agent Instructions — TC-01

## Context

This is the `batch` ring of `trios-train-cpu`, scaffolded for issue #238.

## Files

- `src/lib.rs` — ring entry point (currently a placeholder)
- `Cargo.toml` — workspace member, Bronze tier
- `RING.md` — ring identity and laws
- `TASK.md` — incremental migration checklist

## Allowed

- Add types and functions that belong to the `batch` concern
- Add unit tests under `#[cfg(test)]`
- Re-export types upward to the parent crate's facade

## Forbidden

- Importing sibling rings directly (R1)
- Adding I/O or async runtimes that conflict with parent crate
- Breaking the `ring_id()` contract used by smoke tests
