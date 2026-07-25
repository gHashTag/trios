# AGENTS.md — AH-02 (trios-agent-harness)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: AH-02
- Package: trios-agent-harness-ah02
- Role: Пер-агентная очередь сообщений, порт lib/agents/message-queue

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
