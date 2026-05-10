# AGENTS — BR-IO-CHAT-07

| Agent | Role |
|-------|------|
| OMEGA | ring owner — async timing, tokio integration |
| THETA | reviewer — checks against CR-CHAT-07 pure logic |
| PHI   | falsifier — owns the cover_traffic_correlation category |

## Editing rules

- Logic changes go in `CR-CHAT-07` first, then this ring re-tests the
  async path.
- Never add randomness — covers must be visually-on-the-wire identical
  but deterministically scheduled.
- Tests must stay under `#[tokio::test(start_paused = true)]`.
