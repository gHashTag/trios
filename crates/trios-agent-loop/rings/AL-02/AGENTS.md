# AGENTS.md — AL-02 (trios-agent-loop)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AL-02
- Package: trios-agent-loop-al02
- Role: Движок цикла

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
