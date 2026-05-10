# Agent Notes — BR-OUTPUT-CHAT

This is a re-export ring. Do NOT add logic here. If you need a new type:

1. Add it to a CR-CHAT-* Silver ring with tests.
2. Re-export it from BR-OUTPUT-CHAT in the matching `pub mod`.
3. Bump the parent EPIC checklist (trinity-fpga#28).

If a downstream binary needs `tokio` or `sea-orm`, depend on
`trios-chat-br-io-chat-05` directly — never widen BR-OUTPUT-CHAT's
dependency surface.
