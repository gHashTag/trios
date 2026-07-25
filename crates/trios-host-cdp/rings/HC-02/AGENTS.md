# AGENTS.md — HC-02 (trios-host-cdp)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: HC-02
- Package: trios-host-cdp-hc02
- Role: Поллер

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
