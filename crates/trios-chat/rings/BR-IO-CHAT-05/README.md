# BR-IO-CHAT-05 — SeaORM Postgres backend

Concrete async SeaORM impl of CR-CHAT-05's `Store` trait. Bronze-tier
I/O ring; the only place sea-orm / tokio appear in the trios-chat
graph. See `RING.md` for the contract.

```bash
cargo build  -p trios-chat-br-io-chat-05
DATABASE_URL=postgres://... cargo test -p trios-chat-br-io-chat-05
```

🌻 `φ² + φ⁻² = 3`
