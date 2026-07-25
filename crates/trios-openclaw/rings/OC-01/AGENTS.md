# AGENTS.md — OC-01 (trios-openclaw)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: OC-01
- Package: trios-openclaw-oc01
- Role: Маппинг Hermes-провайдеров, порт lib/agents/hermes/hermes-provider-map

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
