# AGENTS.md — AH-03 (trios-agent-harness)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AH-03
- Package: trios-agent-harness-ah03
- Role: Реестр активных ходов, порт lib/agents/active-turn-registry

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
