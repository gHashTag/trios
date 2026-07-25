# AGENTS.md — BR-OUTPUT (trios-store)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: BR-OUTPUT
- Package: trios-store-br-output
- Role: Assembly ring (single entry point)

## What this ring does

Composes open (ST-01) + migrate (ST-02); re-exports rows and `Store`. No own logic.

## Rules for agents

- Touch only this ring's files; siblings are imported via the parent (R5/R9).
- Keep README/TASK/AGENTS in sync with the code (I5).
