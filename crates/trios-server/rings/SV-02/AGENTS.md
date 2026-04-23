# Agent Instructions — SV-02

## Context
A2A protocol ring. Currently in-memory. Next step: persist to trios-data.

## Verification
```bash
cargo test -p trios-server
# Manual via REST:
# curl -X POST http://localhost:9005/api/chat -d '{"method":"a2a/list_agents"}'
```
