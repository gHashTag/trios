# RING — AH-02 (trios-agent-harness)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-harness-ah02 |
| Sealed | No |

## Purpose

Пер-агентная очередь сообщений, порт lib/agents/message-queue.ts (FileMessageQueue): файловое хранение, FIFO-порядок.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
