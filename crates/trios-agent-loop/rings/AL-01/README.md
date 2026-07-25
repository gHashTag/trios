# RING — AL-01 (trios-agent-loop)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-agent-loop-al01 |
| Sealed | No |

## Purpose

Реестр инструментов и trait `BrowserBridge`: builtin-инструменты (echo, time) + 12 browser_* инструментов, исполняемых через мост в SR-03-очередь.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
