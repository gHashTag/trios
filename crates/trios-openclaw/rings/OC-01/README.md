# RING — OC-01 (trios-openclaw)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-openclaw-oc01 |
| Sealed | No |

## Purpose

Маппинг Hermes-провайдеров, порт lib/agents/hermes/hermes-provider-map.ts: чистые lookup-таблицы моделей/провайдеров.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
