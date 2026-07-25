# RING — BW-01 (trios-browser)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-browser-bw01 |
| Sealed | No |

## Purpose

Протокол действий браузера: goto/goBack/goForward/content/evaluate/click/screenshot и типизированные ответы — общий язык SR-03-очереди и исполнителей.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
