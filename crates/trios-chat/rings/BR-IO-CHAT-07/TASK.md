# TASK — BR-IO-CHAT-07

- [x] Cargo.toml with tokio + CR-CHAT-07 deps
- [x] `WireEmitter` over `mpsc::UnboundedSender<Emission>`
- [x] 5 deterministic async tests (`start_paused = true`)
- [x] Stream-equivalence test vs pure CR-CHAT-07
- [x] Graceful-shutdown on closed channel
- [ ] (future wave) plug into trios-mesh node `wire_pump` task
- [ ] (future wave) BR-IO-CHAT-08 — random-cover-content generator
