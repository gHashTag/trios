# AGENTS.md — ST-02 (trios-store)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: ST-02
- Package: trios-store-st02
- Role: Idempotent DDL migrations

## What this ring does

Applies `CREATE TABLE IF NOT EXISTS` + indexes matching the drizzle schema. No data mutation.

## Rules for agents

- Touch only this ring's files; siblings are imported via the parent (R5/R9).
- Keep README/TASK/AGENTS in sync with the code (I5).
