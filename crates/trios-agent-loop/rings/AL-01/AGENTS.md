# AGENTS.md — AL-01 (trios-agent-loop)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AL-01
- Package: trios-agent-loop-al01
- Role: Реестр инструментов и trait `BrowserBridge`

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
