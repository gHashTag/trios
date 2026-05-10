# CR-CHAT-00 — Chat wire-format primitives

Bottom-of-graph types for Trinity Secure Chat. See `RING.md` for the
ring contract, `AGENTS.md` for invariants, `TASK.md` for status.

```
┌─────────────────────────────────────────────────────┐
│              trios-chat ring graph                  │
│                                                     │
│           ┌──────────────────────┐                  │
│           │     CR-CHAT-00       │  ← you are here  │
│           │    (chat-types)      │                  │
│           └─────────┬────────────┘                  │
│                     │                               │
│        ┌──────┬─────┼─────┬──────┬───────┐          │
│        ▼      ▼     ▼     ▼      ▼       ▼          │
│      C-01   C-02  C-03  C-04   C-05   BR-IO-05      │
│     sealed ratch grp  inj+   persist   SeaORM       │
│                                                     │
└─────────────────────────────────────────────────────┘
```

🌻 `φ² + φ⁻² = 3`
