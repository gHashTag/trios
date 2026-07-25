# RING — AH-03 (trios-agent-harness)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-harness-ah03 |
| Sealed | No |

## Purpose

Реестр активных ходов, порт lib/agents/active-turn-registry.ts: чистая state-machine занятости агентов.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
