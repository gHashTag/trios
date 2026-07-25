# AGENTS.md — SR-04 (trios-a2a)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-04
- Package: trios-a2a-sr04
- Role: Долговременная персистентность A2A-реестра на SeaORM (SQLite)

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
