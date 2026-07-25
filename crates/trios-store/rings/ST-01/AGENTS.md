# AGENTS.md — ST-01 (trios-store)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: ST-01
- Package: trios-store-st01
- Role: SQLite repository over ST-00 rows

## What this ring does

Wraps a SeaORM `DatabaseConnection` (SQLite); typed CRUD for `agent_definitions`, `oauth_tokens`, `produced_files`. Coexists with the TS backend on the same `.db` file.

## Rules for agents

- Touch only this ring's files; siblings are imported via the parent (R5/R9).
- Keep README/TASK/AGENTS in sync with the code (I5).
