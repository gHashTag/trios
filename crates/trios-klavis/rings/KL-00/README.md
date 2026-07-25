# RING — KL-00 (trios-klavis)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-klavis-kl00 |
| Sealed | No |

## Purpose

Базовые типы trios-klavis (скаффолд Волны 0): чистые данные + serde, без I/O и async. Основа для будущих колец интеграций.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
