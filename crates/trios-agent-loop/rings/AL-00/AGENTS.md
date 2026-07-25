# AGENTS.md — AL-00 (trios-agent-loop)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AL-00
- Package: trios-agent-loop-al00
- Role: Типы агентного цикла

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
