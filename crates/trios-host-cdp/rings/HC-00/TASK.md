# TASK — HC-00 (trios-host-cdp)

## Status: DONE ✅

## Completed

- [x] `discover_page_ws(http_base)`
- [x] `CdpClient::connect` + pending-map id→oneshot
- [x] игнорирование событий без id
- [x] 4 теста (шум, ошибки, конкурентные вызовы, discovery)

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
