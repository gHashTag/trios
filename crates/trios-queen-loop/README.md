# trios-queen-loop

Autonomous **Queen** daemon for the Queen↔Doctor closed loop.

```
                    .trinity/queen/
                    ├── policy.json   (god_mode, max_auto_level=3)
                    └── actions.json  (29 catalogued actions)
                              │
                              ▼
   ┌──────────────────────┐     queen/order      ┌──────────────────┐
   │  trios-queen-loop    │ ───────────────────▶ │  trios-server    │
   │  (this crate)        │                       │   /operator WS   │
   │                      │ ◀─────────────────── │                  │
   └──────────────────────┘   BusEvent::         └──────────────────┘
                              DoctorReport               │
                                                         │ broadcast
                                                         ▼
                                                ┌──────────────────┐
                                                │ trios-doctor-    │
                                                │ loop (subscriber)│
                                                └──────────────────┘
```

## What it does

Every `TRIOS_QUEEN_TICK_SECS` (default 60) the Queen daemon:

1. Connects to `ws://127.0.0.1:9005/operator?token=$TRIOS_OPERATOR_TOKEN`
   (URL: `TRIOS_QUEEN_WS_URL`, token: `TRIOS_OPERATOR_TOKEN`).
2. Reads `.trinity/queen/policy.json` and `.trinity/queen/actions.json` from the
   workspace root.
3. Filters actions whose `command` starts with `doctor ` and whose `level`
   ≤ `policy.max_auto_level` (default 3).
4. Picks one deterministically: `(epoch_secs / tick_secs) % candidates.len()`
   — gives stable rotation without persistent state.
5. Sends a `queen/order` RPC frame:
   ```json
   {"jsonrpc":"2.0","id":"<uuid>","method":"queen/order",
    "params":{"action":"doctor scan","target_agent":"doctor",
              "params":{"soul":"SCARABS","tick":42}}}
   ```
6. Tracks the returned `order_id` in a small in-memory ring buffer (`VecDeque<64>`)
   and correlates incoming `BusEvent::DoctorReport` events back to the
   originating order.

If the WS connection drops, the daemon waits `TRIOS_DOCTOR_RECONNECT_SECS`
(default 5) and reconnects — same socket, append-only stream (Constitution L21).

## Environment

| Variable                      | Default                                 |
|-------------------------------|-----------------------------------------|
| `TRIOS_QUEEN_WS_URL`          | `ws://127.0.0.1:9005/operator`          |
| `TRIOS_OPERATOR_TOKEN`        | _(none)_ — required for `/operator`     |
| `TRIOS_QUEEN_TICK_SECS`       | `60`                                    |
| `TRIOS_QUEEN_RECONNECT_SECS`  | `5`                                     |
| `TRIOS_QUEEN_SOUL`            | `SCARABS`                               |
| `TRIOS_QUEEN_WORKSPACE`       | _current dir_ (looks for `.trinity/...`)|

## Run

```bash
cargo run -p trios-queen-loop --bin trios-queen-loop
```

## Constitutional anchors

* φ² + φ⁻² = 3
* **L1**  — pure Rust, no shell scripts.
* **L11** — every order carries a `soul` field (default `SCARABS`).
* **L14** — commit trailer `Agent: SCARABS`.
* **L21** — broadcast bus is append-only.
* **L24** — agents speak only via the canonical bus, never sibling sockets.
