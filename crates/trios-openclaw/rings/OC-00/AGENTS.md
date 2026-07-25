# AGENTS.md — OC-00 (trios-openclaw)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: OC-00
- Package: trios-openclaw-oc00
- Role: Конфиг OpenClaw-гейтвея + сборка ACP-команд, порт lib/agents/openclaw/acp-command

## Rules for agents

1. Не добавлять зависимостей, направленных вверх по кольцам.
2. Любое изменение публичного API — только с обновлением тестов и RING.md крейта.
3. Соблюдать законы репозитория: L1 (без .sh), L3 (clippy clean), L4 (tests pass).
