# RING — OC-00 (trios-openclaw)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-openclaw-oc00 |
| Sealed | No |

## Purpose

Конфиг OpenClaw-гейтвея + сборка ACP-команд, порт lib/agents/openclaw/acp-command.ts. Без VM-рантайма — только чистая сборка команд.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
