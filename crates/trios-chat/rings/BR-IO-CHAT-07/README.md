# BR-IO-CHAT-07 — async wire-emitter

Bronze-tier I/O ring. Drives `CR-CHAT-07::CoverScheduler` over
`tokio::time` to push `Real`/`Cover` emissions onto an
`mpsc::UnboundedSender`. See [`RING.md`](./RING.md) for invariants and
[`AGENTS.md`](./AGENTS.md) for who edits what.

```rust,ignore
use trios_chat_br_io_chat_07::WireEmitter;
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::unbounded_channel();
let mut e = WireEmitter::with_default_tick(tx);
e.enqueue_real();
e.run_for(3).await;
// rx now has [Real, Cover, Cover]
```
