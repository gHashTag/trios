# AGENTS.md — SR-03 (trios-a2a)

> AAIF-compliant | MCP-compatible | BrowserOS server side

## Identity

- Ring: SR-03
- Package: trios-a2a-sr03
- Role: BrowserOS A2A agent — серверные типы и очередь команд

## Что делает этот ring

Определяет типы для протокола BrowserOS:
- Команды от AI агентов → браузеру
- Результаты от браузера → AI агентам
- Очередь с deduplication и TTL

## Rules (ABSOLUTE)

- Читай LAWS.md перед ЛЮБЫМ действием
- R1: только browser command domain
- Нет зависимостей кроме SR-00, SR-01, serde, uuid, chrono
- Нет async/tokio — sync типы
- Нет wasm-bindgen — это серверный код

## Ты МОЖЕШЬ

- ✅ Добавить новые `BrowserCommandType` варианты
- ✅ Добавить новые MCP tool definitions
- ✅ Улучшить `BrowserCommandQueue` логику
- ✅ Добавить тесты

## Ты НЕ МОЖЕШЬ

- ❌ Импортировать SR-02 (избегаем circular deps)
- ❌ Добавлять async/tokio
- ❌ Добавлять wasm-bindgen
- ❌ Делать HTTP запросы — это задача trios-server

## Build

```bash
cargo test -p trios-a2a-sr03
```
