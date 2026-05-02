# TASK — SILVER-RING-DR-04

Closes #462 · Part of #446 · Soul: `Doctor-Doctrine`

## Acceptance

- [x] 8 rules implemented: R-RING-FACADE-001 .. R-COQ-LINK-008
- [x] Every rule emits `Finding { rule, severity, path, message }`
- [x] `EscalationConfig::severity_at(now)` flips `Warn` → `Error` after `t_plus_30`
- [x] Default `t_plus_30 = "2026-06-01T00:00:00Z"` (canonical T+30 for EPIC #446)
- [x] `DoctorConfig::parse` reads `[doctor.escalation]` from `doctor.toml`
- [x] `RuleEngine::run(root, cfg, now)` walks `crates/` once per rule
- [x] 16 unit tests GREEN, clippy 0 warnings
- [x] R-RING-DEP-002: Silver-tier deps only

## Out of scope

- DR-05 facade wiring (separate PR)
- Real-world workspace pass over `gHashTag/trios` (this ring is tested in tempdir fixtures only — pass over the full workspace lands when DR-05 wires in)
- Auto-fix for findings (rules are structural; safe rewrite is hard)
