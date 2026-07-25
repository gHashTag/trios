# AGENTS.md — ST-00 (trios-store)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: ST-00
- Package: trios-store-st00
- Role: Schema row types (bottom of trios-store graph)

## What this ring does

Defines `AgentDefinitionRow`, `OAuthTokenRow`, `ProducedFileRow`. Pure data + serde. No logic, no I/O.

## Rules for agents

- Touch only this ring's files; siblings are imported via the parent (R5/R9).
- Keep README/TASK/AGENTS in sync with the code (I5).
