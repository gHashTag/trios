# RING — AH-00 (trios-agent-harness)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-harness-ah00 |
| Sealed | No |

## Purpose

Базовые типы агентной обвязки, порт 1:1 из TS-бэкенда: дескрипторы сессий, статусы, конверты сообщений. Чистые данные + serde.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
