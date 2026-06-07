# Wave 16-21: Sovereign Scarabs — v3 Control Plane, Matrix Sweep +112

<!-- PR открывается вручную пользователем командой из agent_I_report.md -->
<!-- Branch: wave21/sovereign-scarabs → main -->
<!-- Base commit: d92feb6a9b9a5633ecc4662c927dc06a849caad0 -->

---

## Резюме / Summary

**RU:** Этот PR объединяет работу 6 волн (16–21) над подсистемой `trios-scarab-control` — управляющего контура для Railway-обучения скарабеев. Вводится `bump_strategy_v3` (расширенный whitelist полей), e2e-смоук на живом Railway DSN, матричный план 40+72=112 ячеек, CI-пайплайн с тремя джобами (lint/test/live_smoke) и Rust-бинарник для подготовки ветки.

**EN:** This PR merges 6 waves (16–21) of work on the `trios-scarab-control` subsystem — the Railway training control loop for sovereign scarabs. It introduces `bump_strategy_v3` (widened field whitelist), a live Railway e2e smoke harness, a 40+72=112-cell matrix sweep plan, a 3-job CI pipeline (lint/test/live_smoke), and a Rust binary for branch preparation.

---

## Состав изменений / Scope

### Wave 16 — Base Types + Mock Backend
- `crates/trios-scarab-control/src/strategy.rs` — `Strategy`, `ScarabState` types
- `crates/trios-scarab-control/src/backend.rs` — `Backend` trait
- `crates/trios-scarab-control/src/backend_mock.rs` — in-memory mock

### Wave 17 — PostgreSQL Backend
- `src/backend_pg.rs` — real Postgres via `tokio-postgres`
- `lr` field: `numeric → f64` cast workaround

### Wave 18 — Supervisor Respawn
- `src/supervisor.rs` — respawn on generation change (LISTEN/NOTIFY via `scarab_notify_trg`)

### Wave 19 — Heartbeat Reader + Upsert
- `src/heartbeat.rs` — reads heartbeat files, upserts to DB

### Wave 20 — Prod-Schema Alignment + MCP + Healthz
- **Agent A:** rebase on `d92feb6`, prod-schema fields (`account`, `trainer_bin`, `w_jepa`, `w_nca`)
- **Agent B:** strategy fingerprint + tick fingerprint
- **Agent C:** LISTEN/NOTIFY main loop integration
- **Agent D:** MCP tools for trios-scarab-control
- **Agent E:** Dockerfile, `healthz` endpoint, Railway deployment config

### Wave 21 — v3 Control Plane + CI + Matrix Sweep
- **Agent F/G (v3.sql):** `bump_strategy_v3` — widens UPDATE whitelist to include `trainer_bin`, `w_jepa`, `w_nca`; backward-compatible additive change; `v2` stays live
- **Agent G/H (e2e_railway.rs):** end-to-end smoke test using live `RAILWAY_DSN`; service_id prefix `w21-smoke-*`; full cleanup via `kill_scarab`
- **Agent H (sweep plan):** hybrid matrix 40 champion-deep + 72 grid = 112 cells targeting BPB ≤ 2.20
- **Agent I (CI + PR):** `.github/workflows/wave21_ci.yml` — lint/test/live_smoke/v3_sql_check; this PR body; `agent_I_pr_prep.rs`

---

## Gates / Ворота

### Wave 20 Gates (G1–G7)

| Gate | Описание | Статус |
|------|----------|--------|
| G1 | Rebase на `d92feb6` без конфликтов | ✅ PASS |
| G2 | `cargo test --workspace` (no-DB) | ✅ PASS |
| G3 | prod-schema поля (`account`, `trainer_bin`, `w_jepa`, `w_nca`) | ✅ PASS |
| G4 | LISTEN/NOTIFY через `scarab_notify_trg` | ✅ PASS |
| G5 | MCP tools компилируются | ✅ PASS |
| G6 | Prod delta=0 (snapshot pre/post bpb_samples, scarab_strategy, scarab_dead) | ✅ PASS |
| G7 | Dockerfile + Railway `healthz` endpoint | ✅ PASS |

### Wave 21 Gates (W21-G1..G5)

| Gate | Описание | Статус |
|------|----------|--------|
| W21-G1 | `bump_strategy_v3` SQL синтаксически корректен | ✅ PASS |
| W21-G2 | e2e smoke на live Railway: INSERT → bump_v3 → kill → cleanup | ⏳ PENDING (требует `RAILWAY_DSN` secret в CI) |
| W21-G3 | Matrix sweep plan 112 ячеек задокументирован | ✅ PASS |
| W21-G4 | CI pipeline: lint + test (no-DB) проходят без RAILWAY_DSN | ✅ PASS |
| W21-G5 | `agent_I_pr_prep.rs` компилируется (`cargo check`) | ✅ PASS |

---

## Breaking Changes / Критические изменения

### `bump_strategy_v3` — Breaking-but-Safe-Additive

**RU:** `bump_strategy_v3` расширяет whitelist UPDATE, добавляя поля `trainer_bin`, `w_jepa`, `w_nca`. Это **breaking** в том смысле, что функция является новой (v2 остаётся нетронутой), но **safe-additive** — все вызовы старого кода, использующего `bump_strategy` или `bump_strategy_v2`, продолжают работать без изменений. Миграция применяется напрямую на live Railway (без стейджинга), согласно решению пользователя из Wave 20.

**EN:** `bump_strategy_v3` widens the UPDATE whitelist to include `trainer_bin`, `w_jepa`, `w_nca`. It is **breaking** in the sense that it introduces a new function signature, but **safe-additive** — all callers of `bump_strategy` / `bump_strategy_v2` remain unaffected. Migration applies directly to live Railway (no staging), per Wave 20 user-approved decision.

---

## Артефакты / Artifacts

| Артефакт | Путь | Описание |
|----------|------|----------|
| Combined patch W16-20 | `wave20/wave16_20_combined.patch` | 4047 строк, 5 агентов |
| v3 migration SQL | `wave21/agent_F_v3.sql` (или эквивалент) | `bump_strategy_v3` DDL |
| e2e smoke harness | `wave21/agent_G_e2e_railway.rs` (или эквивалент) | Live Railway тест |
| Matrix sweep plan | `wave21/agent_H_sweep_plan.md` (или эквивалент) | 112 ячеек |
| CI workflow | `wave21/agent_I_ci.yml` → `.github/workflows/wave21_ci.yml` | Lint/test/live_smoke |
| PR prep binary | `wave21/agent_I_pr_prep.rs` | Rust + std::process::Command |
| Agent I report | `wave21/agent_I_report.md` | `gh pr create` команда |

---

## DB Constraints (Hard Rules)

- `seed` CHECK: `{47, 89, 123, 144, 1597, 2584, 4181, 6765, 10946}` only
- `kill_scarab` writes `status='killed'` (NOT `'stop'`)
- `strategy_history`, `scarab_assignment_log`, `scarab_drift` are VIEWS — DELETE forbidden
- Trigger: `scarab_notify_trg`
- Control plane: `ssot.scarab_strategy` UPDATE only — no Railway API, no PAT tokens, no `variableUpsert`

---

## Как применить / How to Apply

```bash
# 1. Запустить agent_I_pr_prep (применяет патч + копирует артефакты W21)
cargo run --manifest-path /home/user/workspace/wave21/agent_I_pr_prep/Cargo.toml

# 2. Проверить ветку
git -C /tmp/trios-w21 log --oneline -5
git -C /tmp/trios-w21 status

# 3. Push + PR (только после явного одобрения пользователя)
gh pr create \
  --repo gHashTag/trios \
  --base main \
  --head wave21/sovereign-scarabs \
  --title "Wave 16-21: sovereign scarabs, v3 control plane, matrix sweep +112" \
  --body-file /home/user/workspace/wave21/agent_I_pr_body.md
```

---

## PhD Timeline

- Защита: 2026-06-15 (T-24 от 2026-05-22)
- Gate-2 target: BPB ≤ 2.20 (текущий чемпион: 2.5586 `IGLA-STRATEGY-gf64-h512-LR0.0001-rng1597-adamw`)
- #446 Matrix: 38/312 измерено (12.2%), план Wave 21 добавляет ещё 112 ячеек

---

_Автоматически подготовлено Agent I · Wave 21 · 2026-05-22_
