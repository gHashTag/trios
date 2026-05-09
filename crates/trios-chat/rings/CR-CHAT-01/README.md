# trios-chat-cr-chat-01 — identity + sealed

First crypto ring of `trios-chat`. Implements **R-CHAT-2** (hybrid PQ
prekey bundle), **R-CHAT-3** (sealed-sender), **R-CHAT-4** (sign only
the bundle, never per-message).

See `RING.md`, `AGENTS.md`, `TASK.md`.

```rust
use trios_chat_cr_chat_01::{Identity, SealedEnvelope};

let alice = Identity::generate();
let bob   = Identity::generate();

let env = SealedEnvelope::seal(
    alice.pre_x25519_secret(),
    &alice.pre_x25519_pub(),
    &bob.pre_x25519_pub(),
    [0u8; 12],
    b"hello bob",
).unwrap();

let plain = env.unseal(bob.pre_x25519_secret(), &bob.pre_x25519_pub()).unwrap();
assert_eq!(plain, b"hello bob");
```

Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`.
