# TASK — AL-02 (trios-agent-loop)

## Status: DONE ✅

## Completed

- [x] `LlmClient` (base_url/api_key/model)
- [x] `run_loop` с max_steps и stop reasons
- [x] события для SSE (assistant_text/tool_call/tool_result/done)
- [x] тесты с мок-LLM

## Context

Кольцо создано в ходе консолидации TS-бэкенда browseros в Rust
(см. docs/TS_RETIREMENT.md). Поведение соответствует исходному TS 1:1,
проверено юнит- и e2e-тестами workspace.
