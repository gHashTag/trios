# AGENTS.md — AH-01 (trios-agent-harness)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AH-01
- Package: trios-agent-harness-ah01
- Role: Каталог агент-адаптеров, порт lib/agents/agent-catalog

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
