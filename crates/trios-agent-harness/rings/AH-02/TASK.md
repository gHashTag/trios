# TASK — AH-02 (trios-agent-harness)

## Status: DONE ✅

## Completed

- [x] файловая очередь
- [x] enqueue/dequeue с сохранением порядка
- [x] порт 1:1 из message-queue.ts
- [x] тесты FIFO и персистентности

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
