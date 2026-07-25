# RING — AH-01 (trios-agent-harness)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-harness-ah01 |
| Sealed | No |

## Purpose

Каталог агент-адаптеров, порт lib/agents/agent-catalog.ts: дескрипторы адаптеров и выбор по имени/способностям.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
