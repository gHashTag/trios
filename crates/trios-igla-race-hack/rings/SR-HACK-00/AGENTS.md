# AGENTS.md — SR-HACK-00

## Invariant I-SCOPE (from AGENTS.md)

**Agents working on this ring MUST only modify files within:**

```
crates/trios-igla-race-hack/rings/SR-HACK-00/
```

Cross-ring refactor requires explicit human authorization.

## Agent Permissions

- ✅ Modify `src/lib.rs` (add types, update existing)
- ✅ Add/update unit tests
- ✅ Update `RING.md` specification
- ✅ Update `TASK.md` for progress tracking
- ❌ DO NOT add business logic (I3 invariant)
- ❌ DO NOT modify files outside this ring

## Soul-name

For this ring: **TermWhisperer** (terminology-focused agent)
