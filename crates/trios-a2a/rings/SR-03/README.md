# RING — SR-03 (trios-a2a)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-a2a-sr03 |
| Sealed | No |

## Purpose

**BrowserOS A2A Agent** — серверная сторона управления браузером.

Этот ring определяет:
- `BrowserCommand` — MCP команда агенту в браузере
- `BrowserResult` — результат выполнения команды
- `BrowserCommandType` — все 12 поддерживаемых операций
- MCP tool definitions для регистрации в trios-server
- `BrowserCommandQueue` — очередь команд ожидающих выполнения

## Место в архитектуре

```
trios-server (SR-02 registry)
    ↓ a2a_browser_command({tool, params})
    ↓ → BrowserCommandQueue (SR-03)

Chrome Extension (EXT-02, WASM)
    ↓ GET /mcp/browser-commands  (poll 2s)
    ↓ ← Vec<BrowserCommand>
    ↓ dispatch → DOM executor (EXT-02/EXT-03)
    ↓ POST /mcp/browser-result
    ↓ → BrowserResult → SR-03 queue update
```

## Почему отдельный ring, не расширение SR-02

SR-02 — общий A2A registry и MCP tools для агентов.
SR-03 — специализированный domain: браузерные команды, очередь,
deduplication, TTL. Разделение по Single Responsibility (R1).

## Laws

- R1: только browser domain типы и очередь
- Нет зависимостей кроме SR-00, SR-01
- Нет tokio/async — чистые sync типы
- Нет WASM — это серверный ring (native Rust)
