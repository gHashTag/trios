# TASK — AL-00 (trios-agent-loop)

## Status: DONE ✅

## Completed

- [x] `ChatMessage` + роли, `ToolDef`/`ToolCall`
- [x] `StopReason` (final/max_steps/error)
- [x] `Usage` (prompt/completion tokens)
- [x] serde round-trip тесты

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
