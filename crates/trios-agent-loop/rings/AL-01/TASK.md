# TASK — AL-01 (trios-agent-loop)

## Status: DONE ✅

## Completed

- [x] `ToolRegistry` с builtin + browser-инструментами
- [x] trait `BrowserBridge` (async, объектно-безопасный)
- [x] маппинг browser_* → BW-01 `BrowserCommand`
- [x] тесты на реестр и диспетчеризацию

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
