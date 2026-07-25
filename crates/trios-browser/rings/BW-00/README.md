# RING — BW-00 (trios-browser)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-browser-bw00 |
| Sealed | No |

## Purpose

Типы браузерного домена, порт 1:1 из apps/server/src/browser/browser.ts: PageInfo, TabInfo, скриншоты. Чистые данные + serde.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
