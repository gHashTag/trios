# AGENTS.md — HC-01 (trios-host-cdp)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: HC-01
- Package: trios-host-cdp-hc01
- Role: Маппинг SR-03 `BrowserCommand` → CDP

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
