# RING — HC-01 (trios-host-cdp)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-host-cdp-hc01 |
| Sealed | No |

## Purpose

Маппинг SR-03 `BrowserCommand` → CDP: Page.navigate, Runtime.evaluate (с JSON-экранированием селекторов), Page.captureScreenshot, Target.create/closeTarget.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
