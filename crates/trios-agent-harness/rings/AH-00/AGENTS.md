# AGENTS.md — AH-00 (trios-agent-harness)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AH-00
- Package: trios-agent-harness-ah00
- Role: Базовые типы агентной обвязки, порт 1

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
