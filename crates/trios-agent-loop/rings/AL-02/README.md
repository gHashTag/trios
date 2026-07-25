# RING — AL-02 (trios-agent-loop)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-loop-al02 |
| Sealed | No |

## Purpose

Движок цикла: OpenAI-совместимый клиент (chat/completions), пошаговый tool-loop с лимитом шагов, транскрипт, стриминговые события.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
