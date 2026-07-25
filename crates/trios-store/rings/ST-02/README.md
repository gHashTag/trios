# RING — ST-02 (trios-store)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-store-st02 |
| Sealed | No |

## Purpose

DDL migrations mirroring the drizzle SQLite schema. Idempotent `CREATE TABLE IF NOT EXISTS` + indexes matching `browseros-agent/apps/server/src/lib/db/schema/*`. Safe to run against an existing TS-created database (all `IF NOT EXISTS`).

## API Surface (pub)

| Item | Role |
|------|------|
| `migrate(pool)` | applies idempotent DDL to a SqlitePool |

## Laws

- R1 / R5 / R9: Ring isolation, no sibling imports, parent re-exports only
- I5: README + TASK + AGENTS present in every ring
