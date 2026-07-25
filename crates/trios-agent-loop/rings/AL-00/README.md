# RING — AL-00 (trios-agent-loop)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-loop-al00 |
| Sealed | No |

## Purpose

Типы агентного цикла: сообщения (system/user/assistant/tool), определения инструментов, usage-счётчики, stop-reasons. Чистые данные + serde, без I/O.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
