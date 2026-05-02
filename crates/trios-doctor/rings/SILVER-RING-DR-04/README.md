# SILVER-RING-DR-04 — ring-architecture lint rules 🥈

**Soul-name:** `Doctor-Doctrine` · **Codename:** `DELTA` · **Tier:** 🥈 Silver · **Kingdom:** Cross-kingdom

> Closes #462 · Part of #446
> Anchor: `φ² + φ⁻² = 3`

## Mission

Encode the eight ring-architecture rules the constitution actually cares about into one lint engine. Every finding ships as `Severity::Warn` until `[doctor.escalation] T_plus_30 = "2026-06-01"`, after which `Severity::Warn` becomes `Severity::Error`.

## Rules

| Id | Rule | Heuristic |
|----|------|-----------|
| `R-RING-FACADE-001` | outer GOLD `src/lib.rs` ≤ 50 LoC, re-exports only | line count + `is_facade_only` predicate |
| `R-RING-DEP-002`    | Silver-tier has no I/O | `tokio` / `reqwest` / `tonic` / `rusqlite` forbidden under `rings/SILVER-*` and `rings/SR-*` |
| `R-RING-FLOW-003`   | Silver may not depend on a Bronze sibling | path dep `../BR-*` or `../BRONZE-RING-*` from a Silver Cargo.toml |
| `R-RING-BR-004`     | Bronze rings re-exposed via parent GOLD facade | outer `src/lib.rs` mentions `_br_` / `br_output` |
| `R-MCP-BRIDGE-005`  | MCP bridge rings live under `vendor/tri-mcp/rings/` | dir name contains `mcp` outside that path |
| `R-L1-ECHO-006`     | no `.sh` files inside `crates/` | extension scan |
| `R-L6-PURE-007`     | no `.py` files inside `crates/` | extension scan |
| `R-COQ-LINK-008`    | every Bronze BPB sink references a Coq theorem id | grep `COQ_THEOREM_ID` / `coq_theorem_id` |

## Surface

```rust
let cfg = EscalationConfig { t_plus_30: DoctorConfig::default_t_plus_30() };
let findings = RuleEngine::new().run(&workspace_root, &cfg, Utc::now())?;
for f in findings {
    println!("[{:?}] {} :: {}", f.severity, f.rule.slug(), f.message);
}
```

## Honored constitutional rules (recursive)

- **R-RING-DEP-002** — Silver-tier deps only: `serde`, `serde_json`, `toml`, `chrono`, `walkdir`, `thiserror`. No tokio, no reqwest, no subprocess.
- **R-RING-FACADE-001** — facade dead-code from gold until DR-05 wires it in.
- **R-L1-ECHO-006** — no `.sh` here.
- **R-L6-PURE-007** — no `.py` here.
- **L13** — single-ring scope.
- **I5** — README.md, TASK.md, AGENTS.md, RING.md, Cargo.toml, src/lib.rs.

## Tests — 16/16 GREEN

```
empty_workspace_has_no_findings
escalation_warn_before_error_after
doctor_config_parses_escalation
is_facade_only_accepts_reexports_and_docs
is_facade_only_rejects_business_logic
r1_accepts_clean_facade
r1_flags_business_logic_in_outer_facade
r2_flags_silver_with_tokio
r3_flags_silver_depending_on_bronze
r4_flags_bronze_without_facade_reexport
r5_flags_mcp_outside_vendor
r6_flags_sh_in_crates
r7_flags_py_in_crates
r8_accepts_bronze_with_coq_link
r8_flags_bronze_bpb_sink_without_coq_link
rule_id_slugs_are_stable
```

`cargo clippy --manifest-path crates/trios-doctor/rings/SILVER-RING-DR-04/Cargo.toml --all-targets -- -D warnings` → **0 warnings**.

## Out of scope (R5-honest)

- Wiring `RuleEngine` into the `trios-doctor` gold facade — that lands in DR-05 alongside the escalation date renderer.
- SARIF / GitHub-Actions output formatting — already lives in DR-03.
- Auto-fix for findings (`cargo fix`-style) — out of scope; the rules are too structural for safe rewrite.
