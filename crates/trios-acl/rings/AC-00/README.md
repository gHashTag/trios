# RING — AC-00 (trios-acl)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-acl-ac00 |
| Sealed | No |

## Purpose

Базовые типы trios-acl (скаффолд Волны 0): модели прав/доступа, чистые данные + serde, без I/O.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
