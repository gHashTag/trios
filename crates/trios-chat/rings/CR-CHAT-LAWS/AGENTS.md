# Agent Notes — CR-CHAT-LAWS

This ring is intentionally tiny and append-only. Do NOT modify the contents of `R_CHAT_LAWS`. Any change requires:

1. ADR-CHAT-NN commit citing rationale
2. Updating `laws_hash` reference in downstream guards
3. Coq proof update if law affects an invariant

If you find yourself wanting to "fix a typo in a law", STOP — laws are immutable text by design.
