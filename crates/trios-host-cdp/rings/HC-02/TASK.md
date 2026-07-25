# TASK — HC-02 (trios-host-cdp)

## Status: DONE ✅

## Completed

- [x] `PollerConfig::from_env` (TRIOS_SERVER_WS, TRIOS_BROWSER_AGENT_ID, TRIOS_POLL_INTERVAL_MS)
- [x] `ws_call` с пропуском event-бродкастов
- [x] `run` с реконнектом
- [x] 2 e2e-теста с фейковым WS-сервером

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
