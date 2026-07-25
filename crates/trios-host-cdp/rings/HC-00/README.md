# RING — HC-00 (trios-host-cdp)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-host-cdp-hc00 |
| Sealed | No |

## Purpose

Сырой CDP-клиент: discovery страницы через GET /json/list, WS-подключение, корреляция запрос/ответ по id, пропуск CDP-событий, таймаут 20с.

## Dependency discipline

Кольцо следует rings-архитектуре консолидации (Волны 0–10, docs/TS_RETIREMENT.md):
нижние кольца — чистые типы, верхние — I/O. Зависимости направлены строго вниз.
