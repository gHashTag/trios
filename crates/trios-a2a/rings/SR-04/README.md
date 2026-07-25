# RING — SR-04 (trios-a2a)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-a2a-sr04 |
| Sealed | No |

## Purpose

Долговременная персистентность A2A-реестра на SeaORM (SQLite): карточки агентов и задачи переживают рестарт trios-server.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
