# trios-chat-cr-chat-04 — padding

Fixed-size padding classes (R-CHAT-9) for Trinity Secure Chat.

See `RING.md` for the full ring contract, `AGENTS.md` for editing
rules, and `TASK.md` for status.

```rust
use trios_chat_cr_chat_04::{pad_class, unpad};
let p = b"hello";
let buf = pad_class(p);
assert_eq!(buf.len(), 256);
assert_eq!(unpad(&buf).unwrap(), p);
```

Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`.
