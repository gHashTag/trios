# TASK — HC-01 (trios-host-cdp)

## Status: DONE ✅

## Completed

- [x] trait `CdpCall` (реализован для CdpClient)
- [x] `execute_command` для всех 12 команд
- [x] обработка exceptionDetails → ошибка
- [x] 6 тестов с FakeCdp

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
