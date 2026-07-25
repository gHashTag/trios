# RING — HC-02 (trios-host-cdp)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-host-cdp-hc02 |
| Sealed | No |

## Purpose

Поллер: WS-клиент к trios-server, цикл browser/poll → исполнение через HC-01 → browser/result, reconnect с backoff, конфиг из env.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
