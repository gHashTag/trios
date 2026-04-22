# Agent Instructions — SV-00

## Context
Server bootstrap ring. Port 9005 is sacred (L5). Never change it without migration.

## Verification
```bash
cargo test -p trios-server
cargo clippy -p trios-server -- -D warnings
```
