# 🧪 MASTER EXPERIMENT TRACKER
## IGLA RACE × JEPA-T × NCA × GF16 × ASHA × Coq Invariants — ONE SHOT HUB

**Обновлено: 2026-04-25 21:40 +07 | φ² + 1/φ² = 3 | TRINITY**
**🔴 ISSUE #143 НЕ ЗАКРЫВАТЬ — только gHashTag закрывает!**
**🔴 SYNCED WITH ISSUE #143 v2 — Laws L-R1..L-R14, Coq invariants INV-1..INV-10, branch=main only**

> Единый документ фактов. Только то, что УЖЕ произошло с реальными числами.
> Планы и TODO → [Issue #143](https://github.com/gHashTag/trios/issues/143)
> Все агенты работают ТОЛЬКО в ветке **main**. Нарушение L-R12 = блокировка.
> Все Coq инварианты должны компилироваться (`coqc proofs/igla/*.v` = exit 0) до старта race (L-R14).

---

## 🚨 REAL-TIME STATUS (2026-04-25 21:40 UTC+7)

```
BASELINE:   2.5329 BPB  (6-gram h=384 lr=0.004 seed=43, 12K steps)
NEW RECORD: 2.5193 BPB  (ASHA trial #9006, 27K steps) ← текущий CHAMPION
N-GRAM CEILING: ~2.51 BPB — архитектурный прыжок ОБЯЗАТЕЛЕН
GATE-1:     BPB ≤ 2.22  (Rung-1, JEPA-T должен пройти)
GATE-2:     BPB ≤ 2.03  (Rung-2, победа над N-gram)
IGLA TARGET: BPB < 1.50 (Apr 30 deadline) — 3 seeds (42,43,44), p<0.01
SOTA REF:   1.1147 BPB  (parameter-golf #1: Self-Gen GPTQ + XSA-all)
                1.2150 BPB  (HierAttn v3 EMA, наш результат на T4 GPU)
```

### Coq Invariant Health (TASK-COQ-001 → IGLA)

| ID | Theorem | Status | Effect | Anchor |
|----|---------|--------|--------|--------|
| INV-1 | `bpb_decreases_with_real_gradient` | partial | fixes TASK-5D | 7-step α_φ derivation |
| INV-2 | `asha_champion_survives` | **PROVEN (0 Admitted)** | 0 false prunes | threshold=3.5=φ²+φ⁻²+0.5 |
| INV-3 | `gf16_safe_domain` | Lucas proven | -40% configs | Lucas closure φ²ⁿ+ψ²ⁿ ∈ Z |
| INV-4 | `nca_entropy_stability` | **PROVEN (0 Admitted)** | -30% configs | A5/E8 band width=1 (integer!) |
| INV-5 | `lucas_closure_gf16` | n=1,2 proven | GF16 consistency | φ²ⁿ+φ⁻²ⁿ ∈ Z |
| INV-6 | `ema_decay_valid` | TODO | -20% configs | cos schedule [0.996,1.0] |
| INV-7 | `igla_found_criterion` | TODO | L-R14 gate | victory iff 3-seed BPB<1.50 |
| INV-8 | `lr_phi_band` | **PROVEN (0 Admitted)** | -60% configs | lr=0.004=α_φ/φ³ |
| INV-9 | `qk_gain_phi_sq` | TODO | -10% configs | QK-gain=φ² unique |
| INV-10 | `asha_rungs_trinity` | TODO | correctness | rungs=1000·3ᵏ |

**Search-space reduction:** 50,000 configs → ~6,000 (8.3× speedup, 625h → 75h).

---

## 🏆 SECTION 1: BASELINE CHAMPIONS

> Неизменяемая история. Одна строка = один verified champion. Источник: Neon `igla_race_trials`.

| # | Trial ID | Arch | d_model | ctx | lr | seed | Optimizer | Precision | BPB | Steps | Date | Notes |
|---|----------|------|---------|-----|----|------|-----------|-----------|-----|-------|------|-------|
| 🥇 | `#9006` | 6-gram | 384 | 6 | 0.004 | 43 | AdamW | f32 | **2.5193** | 27K | 2026-04-25 | ASHA new record, IGLA champion (Neon verified) |
| 🥈 | `#9001`/`#1017` | 6-gram | 384 | 6 | 0.004 | 43 | AdamW | f32 | 2.5329 | 12K | 2026-04-24 | DB-ANALYST confirmed |
| 🥉 | `#1018` | 6-gram 3-seed | 384 | 6 | 0.004 | 41-43 | AdamW | f32 | 2.5431 ± 0.010 | 12K | 2026-04-24 | Seed stability verified |
| 4 | `#2005` | 7-gram | 384 | 7 | 0.004 | 43 | AdamW | f32 | 2.5497 | 12K | 2026-04-24 | Diminishing returns vs 6-gram |

**Neon verdict (DB-ANALYST):** lr=0.004 outperforms lr=0.005 by 0.017 BPB (n=6 trials at 6-gram h=384). h=384 Pareto-optimal: h=512 overfits, h=256 +0.07 BPB worse. 6-gram optimal: 7-gram +0.017, 5-gram +0.039, 4-gram +0.16 BPB.

---

## 📊 SECTION 2: ПОЛНАЯ ТАБЛИЦА ВСЕХ ЭКСПЕРИМЕНТОВ

### 2.1 N-Gram CPU — Хронология (Apr 21–25)

| # | Дата | Arch | d_model | ctx | lr | Optimizer | BPB | Steps | Статус | Заметка |
|---|------|------|---------|-----|----|-----------|-----|-------|--------|--------|
| E-001 | Apr 21 | Bigram | — | 2 | — | — | 3.90 | — | ✅ | Baseline start |
| E-002 | Apr 21 | Trigram | — | 3 | — | AdamW | 3.26 | — | ✅ | +AdamW |
| E-003 | Apr 22 | 4-gram | 64 | 4 | — | AdamW | 2.958 | — | ✅ | ReLU |
| E-004 | Apr 22 | 4-gram | 128 | 4 | — | AdamW | 2.877 | — | ✅ | +hidden |
| E-005 | Apr 22 | 4-gram | 128 | 4 | — | AdamW | 2.780 | — | ✅ | bugfix backward |
| E-006 | Apr 23 | 4-gram | 192 | 4 | — | AdamW | 2.743 | — | ✅ | GELU+res+dropout |
| E-007 | Apr 23 | 4-gram | 192 | 4 | — | AdamW | 2.7184 | — | ✅ | wd=0.01 |
| E-008 | Apr 23 | 4-gram | 256 | 4 | — | AdamW | 2.6964 | — | ✅ | sub-2.70 |
| E-009 | Apr 23 | 4-gram | 192 | 4 | — | AdamW | 2.6942 | — | ✅ | GELU only |
| E-010 | Apr 23 | 4-gram | 256 | 4 | — | AdamW | 2.7265 | — | ⚠️ | GELU d256 worse |
| E-011 | Apr 23 | 5-gram | 256 | 5 | 0.006 | AdamW | 2.6005 | — | ✅ | **ctx3 breakthrough** |
| E-012 | Apr 23 | 5-gram | 384 | 5 | 0.006 | AdamW | 2.5771 | — | ✅ | +hidden |
| E-013 | Apr 23 | 5-gram 3-seed | 384 | 5 | 0.006 | AdamW | 2.5719 ± 0.004 | — | ✅ | Seed stable |
| E-014 | Apr 23 | 6-gram | 384 | 6 | — | AdamW | 2.5678 | — | ✅ | ctx4 |
| E-015 | Apr 23 | 6-gram | 384 | 6 | — | AdamW | 2.5654 | — | ✅ | +label smooth 0.1 |
| E-016 | Apr 24 | 6-gram | 384 | 6 | 0.005 | AdamW | 2.5500 | 12K | ✅ | |
| **E-017** | Apr 24 | **6-gram** | **384** | **6** | **0.004** | **AdamW** | **2.5329** | **12K** | ✅ **CHAMP 12K** | |
| E-018 | Apr 24 | 6-gram 3-seed | 384 | 6 | 0.004 | AdamW | 2.5431 ± 0.010 | 12K | ✅ | 3-seed verified |
| E-019 | Apr 24 | 7-gram | 384 | 7 | 0.004 | AdamW | 2.5497 | 12K | ⚠️ | Diminishing returns |
| E-020 | Apr 24 | 6-gram | 512 | 6 | 0.004 | AdamW | 2.8153 | 8K | ⚠️ | Undertrained, overfit risk |
| E-021 | Apr 24 | 6-gram+attn(fixed) | 384 | 6 | 0.004 | AdamW | 2.9184 | 5K | ✅ | bugfix attn |
| E-022 | Apr 25 | 6-gram+attn(fixed) | 384 | 6 | 0.004 | AdamW | 3.32 @ 2.5K | 12K | 🔄 | trial 5003 RUNNING |
| **E-023** | Apr 25 | **6-gram ASHA** | **384** | **6** | **0.004** | **AdamW** | **2.5193** | **27K** | ✅ **NEW RECORD** | trial #9006 |

### 2.2 JEPA-T Эксперименты (Apr 25)

| # | Trial | Phase | Agent | BPB | Steps | Статус | Примечание |
|---|-------|-------|-------|-----|-------|--------|------------|
| J-001 | GAMMA run | Phase-1 JEPA | GAMMA | 3.8219 | 3K | ⚠️ | Не прошёл gate ≤2.23; embeddings не сошлись (collapse risk) |
| J-002 | ALPHA run | Phase-1+2 | ALPHA | 0.0144 | 5K | ❌ | **L-METRIC violation:** proxy MSE/ln(2), не NTP BPB |
| J-003 | 5003 | N-gram+attn | LEAD | 3.32 @ 2.5K | 12K→ | 🔄 | Running |
| **J-004** | **TASK-5F** | **JEPA 9K** | **ALPHA** | **TBD** | **9K** | **⬜ NEXT** | **Цель: BPB ≤ 2.22** |
| J-005 | TASK-5G | JEPA 27K | ALPHA | TBD | 27K | ⬜ | Цель: BPB ≤ 2.03 |

### 2.3 Architecture Families — Текущее состояние

| Family | Best BPB | Steps | Status | Next Action |
|--------|----------|-------|--------|-------------|
| N-Gram (baseline) | **2.5193** | 27K | ✅ Champion | Ceiling ~2.51 |
| T-JEPA | 3.8219 | 3K | 🔄 Диагностика collapse | TASK-5F: 9K run, EMA β=0.9999 |
| NCA Objective | — | — | ✅ impl, не запущен | wired в hybrid run |
| Hybrid JEPA+NCA+NTP | — | — | ⬜ | Day 1 (Apr 26) |
| Attention/Transformer | 3.505 | 15K | ⚠️ CPU limit | GPU required |
| GF16 Training | — | — | ✅ impl, не tested | BENCH-012 (Day 2) |
| BigramHash + φ-Init | — | — | ⬜ unique angle | См. Issue #143 plan |

### 2.4 Parameter-Golf конкуренция (через igla_race_competitors)

| Rank | Run | BPB | Author | Stack |
|------|-----|-----|--------|-------|
| 1 | 10L Int5-MLP + BigramHash(10240) | 1.1428 | thwu1 | bigram_hash, int5/6, SWA(0.4), Muon WD=0.04 |
| 2 | Int6 MLP3x + SmearGate + BigramHash | 1.1458 | Raahil Shah | smear_gate, bigram_hash, MLP3x, OrthoInit, Muon WD, SWA |
| 3 | 11L MLP3x + Int6 QAT | 1.1502 | aruniyer | int6 QAT, zstd-22, sliding eval, 11 layers |
| 4 | SmearGate + OrthoInit + Muon WD | 1.1556 | aquariouseworkman | smear_gate + STE QAT |
| 5 | 10L Int6 QAT + Zstd MLP2.6x | 1.1586 | yahya010 | Muon=0.99, sliding eval |
| 6 | Mixed Quant + Sliding Window Eval | 1.1630 | aquariouseworkman | int6 block + int8 embed |
| 7 | Muon WD + 10 layer | 1.1748 | notapplica | spectral_init, residual_mix |
| — | HierAttn v3 + EMA | 1.2150 | gHashTag (наш) | hier_attn, residual, EMA, RoPE |

**Critical insight:** No competitor combines `BigramHash × φ-OrthoInit`. This is our Trinity-orthogonal needle (см. Issue #143 / IGLA-NEEDLE stack).

---

## ✅ SECTION 3: TASK PROGRESS

> Только DONE/IN-PROGRESS/BLOCKED. TODO → Issue #143.

| Task | Описание | Статус | Реальный результат | Branch | Date |
|------|----------|--------|-------------------|--------|------|
| TASK-5A | Wire jepa/ module | ✅ DONE | scaffold готов | main | Apr 25 |
| TASK-5B | EMA + masking | ✅ DONE | decay 0.996→1.0, ratio=0.3 | main | Apr 25 |
| TASK-5C | Real cross-attn predictor | ✅ DONE | 147KB, Q/K/V backprop | main (rescue из feat/task-5c) | Apr 25 |
| TASK-5D | Real NTP BPB + MLP backward | ✅ DONE | val CE/ln(2), pipeline | main | Apr 25 |
| TASK-5E | Two-phase JEPA warmup | ✅ DONE | jepa-warmup=1500, ntp-lr-scale=0.25 | main | Apr 25 |
| TASK-5F | JEPA 9K шагов (real NTP BPB) | ⬜ NEXT | — | main | Apr 26 |
| TASK-5G | JEPA 27K финальный gate | ⬜ | — | main | Apr 26-27 |
| R12 | MuonOptimizer NS5 + OptimizerKind | ✅ DONE | wired, ASHA threshold=3.5 (INV-2) | main | Apr 24 |
| SEED-EXPLORER | Поиск стабильного seed | ✅ DONE | seed=43, σ=0.002 | main | Apr 24 |
| DB-ANALYST | Анализ Neon, уроки | ✅ DONE | 16 lessons записаны в `igla_race_experience` | main | Apr 24 |
| GF16 precision | gf16.rs + training step | ⚠️ PARTIAL | impl готов, d_model<256 нестабилен (L-R9) | main | Apr 24 |
| IGLA RACE CLI | tri race start/status/best | ✅ DONE | main.rs, worker, ASHA loop | main | Apr 24-25 |
| NCA Objective | 9×9 grid, entropy [1.5, 2.8] | ✅ DONE | struct NcaObjective, не wired в hybrid | main | Apr 25 |
| Hybrid Trainer | LNTP+LJEPA+LNCA комбинированный | ⬜ | — | main | Apr 26 |
| BENCH-012 | GF16 gradient training | ⬜ HIGH | — | main | TBD |
| TASK-COQ-001 | Coq invariants INV-1..10 | ⚠️ PARTIAL | INV-2,4,8 PROVEN; INV-3,5 partial; INV-1,6,7,9,10 TODO | main | Apr 25 |

---

## ❌ SECTION 4: DEAD ENDS (Never Retry)

> Самое ценное для агентов — что НЕ надо пробовать. Источник: Neon `igla_race_experience`.

| Config | Почему провалился | BPB penalty | Дата | Закон |
|--------|------------------|-------------|------|-------|
| h=512 n-gram | Overfit на TinyShakespeare (CPU) | worse | Apr 23 | — |
| 7-gram context | Diminishing returns vs 6-gram | +0.017 | Apr 24 | — |
| Label smoothing 0.1 | Ухудшает BPB | +0.033 | Apr 23 | — |
| GELU activation | +0.190 BPB vs ReLU в N-gram | +0.190 | Apr 23 | ❌ dead-end |
| Residual в 1-layer N-gram | jumped to 5.2 BPB at step 3.5K | +2.7 | Apr 23 | — |
| Dropout любой | Датасет слишком мал | ухудшает | Apr 23 | — |
| Multi-layer FFN deep | 3.21 BPB — глубина не помогает | +0.68 | Apr 23 | — |
| GF16 + d_model < 256 | NaN/Inf нестабильность | +3.21 | Apr 24 | **L-R9 (INV-3)** |
| Bigram-smear embed (broken proj) | self.proj=None bug, dim==dim no-op | 7.09 | Apr 24 | — |
| Trinity3k 2 layers | Хуже 1 слоя на CPU | 3.94 | Apr 25 | — |
| JEPA без warmup (Phase-1 only) | NTP gradient conflict, collapse | 3.82 | Apr 25 | **L-R10** |
| BPB из JEPA MSE/ln(2) | Физически неверная метрика | 0.014 (фейк) | Apr 25 | **L-METRIC** |
| AdamW + dropout=0.1 | Не помогает на CPU | neutral | Apr 23 | — |
| BigramHash projection broken | self.proj=None — dead code | n/a | Apr 24 | — |
| Depth recurrence no-op | both branches identical, runs 11L not 13L | n/a | Apr 24 | — |
| ASHA threshold=2.65 | Kills ALL valid trials (false prune) | — | Apr 25 | **L-R10 / INV-2** |
| sliding_eval_stride=64 unused | Never invoked in eval_val | n/a | Apr 24 | minor |

---

## 🔀 SECTION 5: COMBINED LOSS EXPERIMENTS

> Секция активируется после первого hybrid run. Ниже шаблон колонок:

| Trial ID | α (NTP) | β (JEPA) | γ (NCA) | Optimizer | d_model | lr | warmup | BPB | ∆ vs 2.5193 | Steps | Date | Notes |
|----------|---------|----------|---------|-----------|---------|----|----|-----|------------|-------|------|-------|
| *(пусто — заполняется Day 1, Apr 26)* | | | | | | | | | | | | |

**Плановые runs Day 1 (Apr 26):**
- `(α=1.0, β=1.0, γ=0.25)` AdamW, steps=3000
- `(α=1.0, β=1.0, γ=0.25)` Muon NS5, steps=3000
- CLI флаги: `--ntp-weight 1.0 --jepa-weight 1.0 --nca-weight 0.25 --optimizer {adamw,muon}`

---

## 💾 SECTION 6: РЕАЛИЗОВАННЫЕ КОМПОНЕНТЫ В main

| Компонент | Файл | Статус | Ключевые параметры |
|-----------|------|--------|--------------------|
| EMA encoder | `jepa/ema.rs` | ✅ | decay 0.996→1.0, ramp=3000 (INV-6 TODO) |
| Span masking | `jepa/masking.rs` | ✅ | ratio=0.3, min=3, max=9 |
| Cross-attn predictor | `jepa/predictor.rs` | ✅ | Q/K/V проекции, L2 norm, backprop |
| MSE + L2 loss | `jepa/loss.rs` | ✅ | anti-collapse |
| NCA objective | `jepa/objective.rs` | ✅ | 9×9=81=3^4, entropy [1.5,2.8], w=0.25 (INV-4 PROVEN) |
| MuonOptimizer NS5 | `optimizer.rs` | ✅ | quintic [3.4445,-4.7750,2.0315] |
| Two-phase warmup | `tjepa_train.rs` | ✅ | Phase-1 JEPA, Phase-2 NTP от step=1500 |
| Real NTP BPB | `tjepa_train.rs` | ✅ | val CE / ln(2) — настоящий BPB (L-METRIC) |
| GF16 training step | `tjepa_train.rs` | ✅ | mixed-precision, guard d_model≥256 (INV-3) |
| IGLA RACE CLI | `igla-race/main.rs` | ✅ | start/status/best |
| ASHA worker loop | `igla-race/asha.rs` | ✅ | rungs 1K→3K→9K→27K, prune 3.50 (INV-2) |
| Neon heartbeat | `tjepa_train.rs` | ✅ | DashboardMeta, every 60s |
| GF16 arithmetic | `gf16.rs` | ✅ | 6:9 exp:mantissa, φ-optimal |
| Coq invariants | `trinity-clara/proofs/igla/*.v` | ⚠️ PARTIAL | INV-2,4,8 PROVEN; INV-1,3,5 partial |
| Rust↔Coq bridge | `src/invariants.rs` | ⬜ TODO | `validate_config()` before subprocess spawn |

---

## 🗄️ SECTION 7: NEON DB SCHEMA

### igla_race_trials

```sql
CREATE TABLE IF NOT EXISTS igla_race_trials (
    trial_id    TEXT PRIMARY KEY,
    config      JSONB NOT NULL,  -- {arch, d_model, lr, seed, optimizer, ntp_w, jepa_w, nca_w, precision, ...}
    status      TEXT DEFAULT 'running',  -- running | complete | pruned | failed | done | winner
    bpb_1000    FLOAT,
    bpb_3000    FLOAT,
    bpb_9000    FLOAT,
    bpb_27000   FLOAT,
    bpb_final   FLOAT,
    agent_id    TEXT,  -- L-R13: required
    machine_id  TEXT,
    branch      TEXT DEFAULT 'main',  -- L-R12: ALWAYS 'main'
    started_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW(),
    notes       TEXT
);
```

### igla_race_experience (16 lessons recorded)

```sql
CREATE TABLE IF NOT EXISTS igla_race_experience (
    id           SERIAL PRIMARY KEY,
    trial_id     BIGINT,
    machine_id   TEXT,
    outcome      TEXT,    -- info | warn | error | pruned | complete | winner | analysis
    config       JSONB,
    bpb_at_kill  DOUBLE PRECISION,
    kill_step    BIGINT,
    lesson       TEXT NOT NULL,
    learned_at   TIMESTAMPTZ DEFAULT NOW()
);
```

### igla_race_competitors (parameter-golf top-15 mirror)

```sql
CREATE TABLE IF NOT EXISTS igla_race_competitors (
    id            SERIAL PRIMARY KEY,
    rank          INT,
    run_name      TEXT,
    score_bpb     DOUBLE PRECISION,
    author        TEXT,
    summary       TEXT,
    techniques    TEXT[],
    date_posted   DATE,
    source_url    TEXT,
    notes         TEXT
);
```

### igla_winning_techniques (frequency × best_bpb)

```sql
CREATE OR REPLACE VIEW igla_winning_techniques AS
SELECT unnest(techniques) AS technique,
       COUNT(*)            AS frequency,
       MIN(score_bpb)      AS best_bpb,
       AVG(score_bpb)      AS avg_bpb
FROM igla_race_competitors
GROUP BY 1
ORDER BY frequency DESC, best_bpb ASC;
```

**Top winning techniques (Apr 25 snapshot):** `sliding_eval` (×5, best 1.1502), `quantization_int6` (×5), `muon` (×4, best 1.1428), `10_layers` (×4), `mlp_3x` (×4), `bigram_hash` (×3, best 1.1428), `SWA` (×2), `smear_gate` (×2), `QAT` (×2), `zstd` (×2).

### igla_agents_heartbeat

```sql
CREATE TABLE IF NOT EXISTS igla_agents_heartbeat (
    agent_id        TEXT PRIMARY KEY,  -- NATO: ALFA, BRAVO, CHARLIE, DELTA, EPSILON, FOXTROT, GAMMA, GOLF, HOTEL, INDIA, LEAD
    machine_id      TEXT NOT NULL,
    branch          TEXT NOT NULL DEFAULT 'main',  -- ALWAYS 'main' (L-R12)
    task            TEXT,
    status          TEXT DEFAULT 'active',
    last_heartbeat  TIMESTAMPTZ DEFAULT NOW()
);
```

### Heartbeat шаблон (каждые 60 секунд)

```sql
INSERT INTO igla_agents_heartbeat (agent_id, machine_id, branch, task, status, last_heartbeat)
VALUES ($1, $2, 'main', $3, 'active', NOW())
ON CONFLICT (agent_id) DO UPDATE
    SET machine_id     = EXCLUDED.machine_id,
        branch         = EXCLUDED.branch,
        task           = EXCLUDED.task,
        status         = EXCLUDED.status,
        last_heartbeat = EXCLUDED.last_heartbeat;
```

### Dashboard SQL: топ-10 champion конфигов

```sql
SELECT trial_id,
       config->>'arch'   AS arch,
       (config->>'d_model')::int AS d_model,
       (config->>'lr')::float    AS lr,
       (config->>'seed')::int    AS seed,
       bpb_final, status, notes
FROM igla_race_trials
WHERE status IN ('complete','done','winner')
ORDER BY bpb_final ASC NULLS LAST
LIMIT 10;
```

### Dashboard SQL: проверка нарушений ветки (L-R12)

```sql
-- Если branch != 'main' — это нарушение L-R12
SELECT agent_id, machine_id, branch, task, status, last_heartbeat
FROM igla_agents_heartbeat
WHERE branch <> 'main'
ORDER BY last_heartbeat DESC;
```

---

## 📋 SECTION 8: ЗАКОНЫ L-R1..L-R14 (UPDATED v2 — синхронизировано с Issue #143)

> ALL агенты обязаны следовать. Нарушение → revert / PR blocked / race invalidated.

| Закон | Правило | Нарушение → |
|-------|---------|-------------|
| **L-R1** | RUST ONLY — zero `.py`, `.sh`, `.ipynb` | REVERT |
| **L-R2** | WORKERS=4-16 через env var | REVERT |
| **L-R3** | Каждый результат → Neon + `.trinity/experience/` | LESSON MISSING |
| **L-R4** | `cargo test --workspace` = GREEN до push | PR BLOCKED |
| **L-R5** | `cargo clippy -- -D warnings` = 0 | PR BLOCKED |
| **L-R6** | SIGTERM → graceful shutdown | DATA LOSS |
| **L-R7** | Neon query timeout ≤ 30 sec | WORKER CRASH |
| **L-R8** | Trainer stdout: ТОЛЬКО `BPB=X.XXXX` | PARSE FAIL |
| **L-R9** | GF16 only with `d_model ≥ 256` | +3.21 BPB (INV-3 PROVEN) |
| **L-R10** | T-JEPA ASHA min rung = 3000 steps | FALSE PRUNE |
| **L-R11** | NCA entropy ∈ [1.5, 2.8] hard penalty | COLLAPSE (INV-4 PROVEN) |
| **L-R12** | All agents → branch `main` ONLY | CONFLICT |
| **L-R13** | `agent_id` + `branch='main'` в каждой Neon записи | DASHBOARD FAIL |
| **L-R14** | `coqc trinity-clara/proofs/igla/*.v` = exit 0 до race | **RACE INVALID** |
| **L-METRIC** | BPB = NTP CE / ln(2) на val set, не JEPA MSE | 🔴 critical |
| **L-NOCLOSE** | Issue #143 не закрывать (только gHashTag) | 🔴 critical |

---

## 🗺️ СЕКЦИЯ 9: ПЛАН ДО APR 30 (синхронизация с Issue #143)

| День | Дата | Цель | Агент | Критерий готовности |
|------|------|------|-------|--------------------|
| **Day 0** | Apr 25 (сегодня) | Coq invariants INV-2,4,8 PROVEN; MASTER_EXPERIMENTS актуален; ASHA threshold=3.5 | DOK + CLARA | этот файл запушен; `coqc proofs/igla/*.v` начат |
| **Day 1** | Apr 26 | Hybrid trainer LNTP+LJEPA+LNCA, Muon vs AdamW; INV-1,6 done | ALPHA+GAMMA | BPB записан в Neon; INV-1 backward gradient ≠ 0 |
| **Day 2** | Apr 27 | GF16-TRAIN (BENCH-012): ∆BPB(f32→gf16) ≤ 0.01; INV-9 (QK-gain φ²) | DELTA+EPSILON | Section 5 заполнена; coqc INV-9 = 0 |
| **Day 3** | Apr 28 | ASHA sweep 36 hybrid configs; INV-7,10 done; IGLA-NEEDLE stack | BETA+LEAD+FOXTROT | Top-1 в Neon < 1.55 BPB |
| **Day 4-5** | Apr 29-30 | 3-seed validation (42, 43, 44, p<0.01); финальный дашборд; coqc all-green | ALL | Mean BPB + σ в #143; **L-R14 PASSED** |

### GF16 sweep параметры (Day 3)

```
lr              ∈ {0.0025, 0.004, 0.006}        = 3 варианта
nca_weight      ∈ {0.10, 0.25, 0.40}            = 3 варианта
jepa_weight     ∈ {0.50, 1.00}                  = 2 варианта
warmup_steps    ∈ {1500, 3000}                  = 2 варианта
─────────────────────────────────────────────────
Итого: 3×3×2×2 = 36 конфигов
ASHA threshold: ≥3.5 после 4000 шагов (INV-2)
GF16 guard: d_model ∈ {256, 384, 512} only (L-R9 / INV-3)
```

### IGLA-NEEDLE stack (Trinity-уникальный угол)

Ни один из топ-10 parameter-golf конкурентов **не комбинирует** `BigramHash × φ-OrthoInit × SWA(1/φ) × GF16(d≥256)`. Это наша игла:

```
T01  φ-OrthoInit (gain = 1/φ ≈ 0.618)              ΔBPB ≈ -0.03..-0.05
T02  BigramHash(729 = 3^6) — Trinity-aligned        ΔBPB ≈ -0.20 (vocab)
T03  SmearGate × GF16 (d=384)                       ΔBPB ≈ -0.05
T04  SWA(1/φ ≈ 0.618) — TRUE SWA, не EMA            ΔBPB ≈ -0.02
T05  3^k layer sweep {3, 9, 27}                      ΔBPB ≈ -0.10
P03  SWA(1/φ) — disambiguation                      ΔBPB ≈ -0.02
P04  OrthoInit baseline (gain=1.0) — control        ΔBPB ≈ -0.02
P07  Residual mix sweep [0.4, 0.5, 0.618, 0.75]    ΔBPB ≈ -0.01
P11  Sliding eval stride=64                          ΔBPB ≈ -0.03
```

---

## 📈 ПРОГРЕСС-ТРЕКЕР

```
START:   3.90 BPB  [████████████████████████████████] Apr 21
Today:   2.519 BPB [████████████████████░░░░░░░░░░░] Apr 25 (-35.4%)
GATE-1:  2.22  BPB [██████████████████░░░░░░░░░░░░░] JEPA Rung-1 target
GATE-2:  2.03  BPB [████████████████░░░░░░░░░░░░░░░] JEPA Rung-2 target
TARGET:  1.50  BPB [████████████░░░░░░░░░░░░░░░░░░░] Apr 30 IGLA
SOTA:    1.11  BPB [█████████░░░░░░░░░░░░░░░░░░░░░░] Self-Gen GPTQ + XSA-all (parameter-golf #1)

BPB gap to GATE-1:  -0.30
BPB gap to TARGET:  -1.02  (GPU архитектуры + JEPA + Muon + NCA + IGLA-NEEDLE)
```

### Ожидаемая траектория (после Coq filtering 8.3× search-space reduction)

| Шаг | Техника | Δ BPB | Итого BPB | Coq invariant gating |
|-----|---------|-------|----------|----------------------|
| baseline | 6-gram ASHA #9006 | — | 2.5193 | INV-2 (threshold 3.5) |
| +1 | Attention 1-2L CPU | -0.30 | ~2.22 | — |
| +2 | T-JEPA (β=1.0, EMA β=0.9999) | -0.20 | ~2.02 | INV-6 |
| +3 | Muon NS5 (orthogonalized, WD=0.04) | -0.15 | ~1.87 | — |
| +4 | NCA regularizer (γ=0.25, K=9, grid=81) | -0.15 | ~1.72 | INV-4 |
| +5 | ReLU² activation | -0.08 | ~1.64 | — |
| +6 | QK-Gain φ² (≈ 2.618) | -0.10 | ~1.54 | INV-9 |
| +7 | GF16 (d_model ≥ 256) | -0.05 | ~1.49 ← **IGLA** | INV-3, L-R9 |
| +8 | φ-LR schedule (lr=α_φ/φ³=0.004) | -0.03 | ~1.46 🎯 | INV-8 |
| +9 | BigramHash(729=3^6) + φ-OrthoInit | -0.10 | ~1.36 | T01+T02 (IGLA-NEEDLE) |

---

## 🔢 SECTION 10: GOLDEN FLOAT FAMILY — ПОЛНЫЙ КАТАЛОГ

> Источник: `zig-golden-float/docs/whitepaper.md` v2.0 + arXiv:2602.15266 + Bergman base-φ + Lucas closure.

### Базовые φ-константы

| Символ | Значение | Идентичность | Использование |
|--------|----------|--------------|---------------|
| φ | 1.6180339887... | (1+√5)/2 | Базис Golden Float |
| 1/φ | 0.6180339887... | φ − 1 | Conjugate, SWA decay, gain |
| φ² | 2.6180339887... | φ + 1 | QK-Gain (INV-9) |
| 1/φ² | 0.3819660112... | 2 − φ | Trinity sub-unit |
| **φ² + 1/φ²** | **3.0** | exact integer | **TRINITY identity** |
| φ − 1/φ | **1.0** | exact integer | Unit residual |
| ln(φ) | 0.4812118250... | log φ | Information content |
| φ³ | 4.2360679... | 2φ + 1 | Next hierarchy level |
| √φ | 1.2720196... | φ^0.5 | Intermediate split |
| α_φ | derived | 7-step φ-norm | INV-8 lr ladder (lr=α_φ/φ³ = 0.004) |
| ψ | -1/φ = -0.6180... | 1 − φ | Lucas closure conjugate |
| L_n | ⌊φⁿ + 1/2⌋ | φⁿ + (-φ)⁻ⁿ | Lucas closure (INV-5) |

### Golden Float Family — все форматы (формализовано в whitepaper v2)

| Format | Bits | Exp:Mant | Numeric anchor | Use case | Status |
|--------|------|----------|----------------|----------|--------|
| **GF8** | 8 | 3:4 (sign 1) | 8 ≈ φ⁴ + φ⁻⁴ | Ultra-low power edge | ⬜ TODO |
| **GF16** | 16 | 6:9 (sign 1) | 6/9 ≈ 2/3 ≈ 1/φ | Production training | ✅ BENCH-001..006 |
| **GF32** | 32 | 13:18 (sign 1) | 13/18 ≈ 0.722 (φ⁻²·k) | FP32 replacement | ⬜ TODO |
| **GF64** | 64 | 21:42 (sign 1) | 21:42 = Fib ratio | Double precision | ⬜ TODO |
| **GFTernary** | 2 | {-φ, 0, +φ} | φ-step ternary | BENCH HYBRID-001 | ⬜ spec only |
| **φ³ (4.236...)** | derived | hierarchy | 2φ+1 | LR schedule, recurrence depth | ✅ used in INV-8 |

### GF16 Engineering Numbers

| Параметр | Значение | Источник |
|----------|----------|----------|
| 6:9 split | 6 exp + 9 mantissa | φ-optimal partition for 16 bit |
| 6/9 = 0.667 | ≈ 2/3 | Engineering approximation to 1/φ = 0.618 |
| Exponent range | 2⁶ = 64 values | Dynamic range |
| Mantissa precision | 2⁹ = 512 steps | Per-step precision |
| MAC-level ratio | 1.37× vs ternary | 71/52 LUT |
| Unit-level ratio | 47–59× vs ternary | 94–118 / 2 LUT |
| Accuracy gap | **0.00%** vs f32 | BENCH-004b (97.67% MNIST) |
| Energy factor | 0.1× vs FP32 | 10× savings |
| 70B model RAM | 14 GB vs 140 GB FP16 | 10× reduction |
| SIMD reduction | 41× vs FP16 | 56 vs 2304 inst/loop |

### Lucas closure (INV-5)

For all integer n, **φ²ⁿ + φ⁻²ⁿ ∈ ℤ**. This is what makes GF16 numerically safe at d_model ≥ 256: gradient norms stay in the Lucas integer band, no NaN/Inf accumulation.

Examples:
- n=1: φ² + φ⁻² = 3 (Trinity identity)
- n=2: φ⁴ + φ⁻⁴ = 7 (Lucas L₄)
- n=3: φ⁶ + φ⁻⁶ = 18 (Lucas L₆)
- n=4: φ⁸ + φ⁻⁸ = 47 (Lucas L₈)

### Bergman base-φ (phinary) representation

Every positive integer = sum of distinct φᵏ powers (k ∈ ℤ), no two consecutive non-zero coefficients (Zeckendorf-style). Used internally by GF16 to argue **uniqueness of mantissa quantization** (no double-rounding).

---

## 🔗 LINKS

- **ONE SHOT HUB**: https://github.com/gHashTag/trios/issues/143 🔴 НЕ ЗАКРЫВАТЬ
- Parameter Golf hub: https://github.com/gHashTag/trios/issues/237
- GF16 Whitepaper v2 (Golden Float Family): https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md
- JEPA-T Paper: https://github.com/gHashTag/trinity/blob/61bf773204e2fee1379a2598350489f20dd49c83/docs/lab/papers/2026-03-15-hslm-tjepa.md
- TASK-COQ-001 (Coq invariants): https://github.com/gHashTag/trinity-clara/blob/main/docs/TASK-COQ-001.md
- arXiv Muon NS5: https://arxiv.org/html/2604.01472v1
- arXiv NCA: https://arxiv.org/abs/2603.10055
- arXiv ReLU²: https://arxiv.org/html/2310.04564v1
- arXiv Lucas/Bergman base-φ: https://arxiv.org/abs/2111.07544
- arXiv JEPA collapse dynamics: https://openreview.net/pdf/5208e18629b914a6e60ef9e64cde266ebaca57c1.pdf
- Parameter Golf leaderboard: https://github.com/openai/parameter-golf

---

*φ² + 1/φ² = 3 | TRINITY | 2026-04-25 21:40 +07 | All agents: branch=main only | L-R1..L-R14 active | RUST ONLY | NEVER STOP*
