# Agent Instructions — LM-00

## Context
LLM client ring. Never hardcode API keys. Use env vars only.

## Forbidden
- No API keys in source
- No panic on network error — return Err

## Verification
```bash
cargo test -p trios-llm
cargo clippy -p trios-llm -- -D warnings
```
