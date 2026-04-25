# 🧪 MASTER EXPERIMENT TRACKER
## IGLA RACE × JEPA-T × NCA × GF16 × ASHA — ONE SHOT HUB

**Обновлено: 2026-04-25 20:00 +07 | φ² + 1/φ² = 3 | TRINITY**
**🔴 ISSUE #143 НЕ ЗАКРЫВАТЬ — только gHashTag закрывает!**

> Единый документ фактов. Только то, что УЖЕ произошло с реальными числами.
> Планы и TODO → [Issue #143](https://github.com/gHashTag/trios/issues/143)
> Все агенты работают ТОЛЬКО в ветке **main**. Нарушение = блокировка.

---

## 🚨 REAL-TIME STATUS (2026-04-25 20:00 UTC+7)

```
BASELINE:   2.5329 BPB  (6-gram h=384 lr=0.004 seed=43, 12K steps)
NEW RECORD: 2.5193 BPB  (ASHA trial #9006, 27K steps) ← текущий CHAMPION
N-GRAM CEILING: ~2.51 BPB — архитектурный прыжок ОБЯЗАТЕЛЕН
GATE-1:     BPB ≤ 2.22  (Rung-1, JEPA-T должен пройти)
GATE-2:     BPB ≤ 2.03  (Rung-2, победа над N-gram)
IGLA TARGET: BPB < 1.50 (Apr 30 deadline)
SOTA REF:   1.2150 BPB  (HierAttn v3, T4 GPU)
```

---

## 🏆 SECTION 1: BASELINE CHAMPIONS

> Неизменяемая история. Одна строка = один verified champion.

| # | Trial ID | Arch | d_model | ctx | lr | seed | Optimizer | Precision | BPB | Steps | Date | Notes |
|---|----------|------|---------|-----|----|------|-----------|-----------|-----|-------|------|-------|
| 🥇 | `#9006` | 6-gram | 384 | 6 | 0.004 | 43 | AdamW | f32 | **2.5193** | 27K | 2026-04-25 | ASHA new record, IGLA champion |
| 🥈 | `#1017` | 6-gram | 384 | 6 | 0.004 | 43 | AdamW | f32 | 2.5329 | 12K | 2026-04-24 | DB-ANALYST confirmed |
| 🥉 | `#1018` | 6-gram 3-seed | 384 | 6 | 0.004 | 41-43 | AdamW | f32 | 2.5431 ± 0.010 | 12K | 2026-04-24 | Seed stability verified |
| 4 | `seed43-r1` | 6-gram | 384 | 6 | 0.004 | 43 | AdamW | f32 | 2.5329 | 12K | 2026-04-24 | SEED-EXPLORER baseline |

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
| J-001 | GAMMA run | Phase-1 JEPA | GAMMA | 3.8219 | 3K | ⚠️ | Не прошёл gate ≤2.23; причина: JEPA embeddings не сошлись |
| J-002 | ALPHA run | Phase-1+2 | ALPHA | 0.0144 | 5K | ❌ | ПОДОЗРИТЕЛЬНО: proxy loss, не NTP BPB |
| J-003 | 5003 | N-gram+attn | LEAD | 3.32 @ 2.5K | 12K→ | 🔄 | Running, ждём результат |
| **J-004** | **TASK-5F** | **JEPA 9K** | **ALPHA** | **TBD** | **9K** | **⬜ NEXT** | **Цель: BPB ≤ 2.22** |
| J-005 | TASK-5G | JEPA 27K | ALPHA | TBD | 27K | ⬜ | Цель: BPB ≤ 2.03 |

### 2.3 Architecture Families — Текущее состояние

| Family | Best BPB | Steps | Status | Next Action |
|--------|----------|-------|--------|-------------|
| N-Gram (baseline) | **2.5193** | 27K | ✅ Champion | Ceiling ~2.51 |
| T-JEPA | 3.8219 | 3K | 🔄 Диагностика | TASK-5F: 9K run |
| NCA Objective | — | — | ✅ impl, не запущен | wired в hybrid run |
| Hybrid JEPA+NCA+NTP | — | — | ⬜ | Day 1 (Apr 26) |
| Attention/Transformer | 3.505 | 15K | ⚠️ CPU limit | GPU required |
| GF16 Training | — | — | ✅ impl, не tested | Day 2 validation |

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
| R12 | MuonOptimizer NS5 + OptimizerKind | ✅ DONE | wired, ASHA threshold=3.5 | main | Apr 24 |
| SEED-EXPLORER | Поиск стабильного seed | ✅ DONE | seed=43, σ=0.002 | main | Apr 24 |
| DB-ANALYST | Анализ Neon, уроки | ✅ DONE | 3 lessons записаны | main | Apr 24 |
| GF16 precision | gf16.rs + training step | ⚠️ PARTIAL | impl готов, d_model<256 нестабилен | main | Apr 24 |
| IGLA RACE CLI | tri race start/status/best | ✅ DONE | main.rs, worker, ASHA loop | main | Apr 24-25 |
| NCA Objective | 9×9 grid, entropy [1.5, 2.8] | ✅ DONE | struct NcaObjective, не wired в hybrid | main | Apr 25 |
| Hybrid Trainer | LNTP+LJEPA+LNCA комбинированный | ⬜ | — | main | Apr 26 |
| BENCH-012 | GF16 gradient training | ⬜ HIGH | — | main | TBD |

---

## ❌ SECTION 4: DEAD ENDS (Never Retry)

> Самое ценное для агентов — что НЕ надо пробовать.

| Config | Почему провалился | BPB penalty | Дата | Закон |
|--------|------------------|-------------|------|-------|
| h=512 n-gram | Overfit на TinyShakespeare (CPU) | worse | Apr 23 | — |
| 7-gram context | Diminishing returns vs 6-gram | +0.017 | Apr 24 | — |
| Label smoothing 0.1 | Ухудшает BPB | +0.033 | Apr 23 | — |
| GELU activation | +0.190 BPB vs ReLU в N-gram | +0.190 | Apr 23 | ❌ dead-end |
| Residual connections | Не помогает N-gram архитектуре | neutral | Apr 23 | — |
| Dropout любой | Датасет слишком мал | ухудшает | Apr 23 | — |
| Multi-layer FFN deep | 3.21 BPB — глубина не помогает | +0.68 | Apr 23 | — |
| GF16 + d_model < 256 | NaN/Inf нестабильность | +3.21 | Apr 24 | **L-R9** |
| Bigram-smear embed | CPU FAILED error | 7.09 | Apr 24 | — |
| Trinity3k 2 layers | Хуже 1 слоя на CPU | 3.94 | Apr 25 | — |
| JEPA без warmup | NTP gradient conflict | 3.82 | Apr 25 | **L-R10** |
| BPB из JEPA MSE/ln(2) | Физически неверная метрика | 0.014 (фейк) | Apr 25 | **L-METRIC** |
| AdamW + dropout=0.1 | Не помогает на CPU | neutral | Apr 23 | — |

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
| EMA encoder | `jepa/ema.rs` | ✅ | decay 0.996→1.0, ramp=3000 |
| Span masking | `jepa/masking.rs` | ✅ | ratio=0.3, min=3, max=9 |
| Cross-attn predictor | `jepa/predictor.rs` | ✅ | Q/K/V проекции, L2 norm, backprop |
| MSE + L2 loss | `jepa/loss.rs` | ✅ | anti-collapse |
| NCA objective | `jepa/objective.rs` | ✅ | 9×9=81=3^4, entropy [1.5,2.8], w=0.25 |
| MuonOptimizer NS5 | `optimizer.rs` | ✅ | quintic [3.4445,-4.7750,2.0315] |
| Two-phase warmup | `tjepa_train.rs` | ✅ | Phase-1 JEPA, Phase-2 NTP от step=1500 |
| Real NTP BPB | `tjepa_train.rs` | ✅ | val CE / ln(2) — настоящий BPB |
| GF16 training step | `tjepa_train.rs` | ✅ | mixed-precision, guard d_model≥256 |
| IGLA RACE CLI | `igla-race/main.rs` | ✅ | start/status/best |
| ASHA worker loop | `igla-race/asha.rs` | ✅ | rungs 1K→3K→9K→27K, prune 3.50 |
| Neon heartbeat | `tjepa_train.rs` | ✅ | DashboardMeta, every 60s |
| GF16 arithmetic | `gf16.rs` | ✅ | 6:9 exp:mantissa, φ-optimal |

---

## 🗄️ SECTION 7: NEON DB SCHEMA

### igla_race_trials

```sql
CREATE TABLE IF NOT EXISTS igla_race_trials (
    trial_id    TEXT PRIMARY KEY,
    config      JSONB NOT NULL,  -- {arch, d_model, lr, seed, optimizer, ntp_w, jepa_w, nca_w, precision, ...}
    status      TEXT DEFAULT 'running',  -- running | complete | pruned | failed
    bpb_1000    FLOAT,
    bpb_3000    FLOAT,
    bpb_9000    FLOAT,
    bpb_27000   FLOAT,
    bpb_final   FLOAT,
    agent_id    TEXT,
    machine_id  TEXT,
    branch      TEXT DEFAULT 'main',
    started_at  TIMESTAMPTZ DEFAULT NOW(),
    updated_at  TIMESTAMPTZ DEFAULT NOW(),
    notes       TEXT
);
```

### igla_race_experience

```sql
CREATE TABLE IF NOT EXISTS igla_race_experience (
    id          SERIAL PRIMARY KEY,
    lesson      TEXT NOT NULL,
    impact      TEXT,  -- high | medium | low
    trial_ref   TEXT,
    created_at  TIMESTAMPTZ DEFAULT NOW()
);
```

### igla_agents_heartbeat ← ДОБАВИТЬ ЕСЛИ НЕТ

```sql
CREATE TABLE IF NOT EXISTS igla_agents_heartbeat (
    agent_id        TEXT PRIMARY KEY,  -- NATO: ALFA, BRAVO, CHARLIE, DELTA, EPSILON, GAMMA, LEAD
    machine_id      TEXT NOT NULL,     -- mac-studio-1, mac-studio-2, macbook-pro-1, partner-1
    branch          TEXT NOT NULL DEFAULT 'main',  -- ВСЕГДА 'main'!
    task            TEXT,              -- TASK-5F, R12, GF16-TRAIN, HYBRID-DAY1
    status          TEXT DEFAULT 'active',  -- active | idle | error
    last_heartbeat  TIMESTAMPTZ DEFAULT NOW()
);
```

### Heartbeat шаблон (каждые 60 секунд)

```sql
INSERT INTO igla_agents_heartbeat (agent_id, machine_id, branch, task, status, last_heartbeat)
VALUES ($1, $2, 'main', $3, 'active', NOW())
ON CONFLICT (agent_id) DO UPDATE
    SET machine_id     = EXCLUDED.machine_id,
        branch         = EXCLUDED.branch,  -- ВСЕГДА 'main'
        task           = EXCLUDED.task,
        status         = EXCLUDED.status,
        last_heartbeat = EXCLUDED.last_heartbeat;
-- $1 = NATO id (ALFA/BRAVO/...), $2 = mac-studio-1, $3 = TASK-5F
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
WHERE status = 'complete'
ORDER BY bpb_final ASC
LIMIT 10;
```

### Dashboard SQL: проверка нарушений ветки

```sql
-- Если branch != 'main' — это нарушение закона L-BRANCH
SELECT agent_id, machine_id, branch, task, status, last_heartbeat
FROM igla_agents_heartbeat
ORDER BY last_heartbeat DESC;
```

---

## 📋 SECTION 8: ЗАКОНЫ (краткий snapshot)

> Полный список → [Issue #143 Laws L-R1..L-R12](https://github.com/gHashTag/trios/issues/143)

| Закон | Правило | Критично |
|-------|---------|----------|
| **L-R1** | RUST ONLY — zero `.py`, `.sh` | ✅ всегда |
| **L-R3** | Каждый результат → Neon + `.trinity/experience/` | ✅ всегда |
| **L-R4** | `cargo test --workspace` = GREEN до push | ✅ всегда |
| **L-R8** | Trainer stdout = ТОЛЬКО `BPB=X.XXXX` | ✅ всегда |
| **L-R9** | GF16 только при `d_model ≥ 256` | 🔴 критично |
| **L-R10** | JEPA ASHA budget ≥ 3000 steps Rung-1 (1.4× медленнее) | 🔴 критично |
| **L-R11** | NCA entropy band [1.5, 2.8] — hard penalty | 🔴 критично |
| **L-BRANCH** | Только ветка **main**, без checkout на хэши | 🔴 критично |
| **L-METRIC** | BPB = NTP CE / ln(2) на val set — не JEPA MSE | 🔴 критично |
| **L-NOCLOSE** | Issue #143 не закрывать | 🔴 критично |

---

## 🗺️ СЕКЦИЯ 9: ПЛАН ДО APR 30 (синхронизация с Issue #143)

| День | Дата | Цель | Агент | Критерий готовности |
|------|------|------|-------|--------------------|
| **Day 0** | Apr 25 (сегодня) | Git sync + MASTER_EXPERIMENTS актуален | DOK | Этот файл запушен |
| **Day 1** | Apr 26 | Hybrid trainer: LNTP+LJEPA+LNCA, Muon vs AdamW | ALPHA+GAMMA | BPB записан в Neon |
| **Day 2** | Apr 27 | GF16-TRAIN: ∆BPB(f32→gf16) ≤ 0.01 | DELTA | Section 5 заполнена |
| **Day 3** | Apr 28 | ASHA sweep 36 конфигов hybrid | BETA+LEAD | Top-1 в Neon |
| **Day 4-5** | Apr 29-30 | 3-seed validation, финальный дашборд | ALL | Mean BPB + σ в Issue #143 |

### GF16 sweep параметры (Day 3)
```
lr              ∈ {0.0025, 0.004, 0.006}        = 3 варианта
nca_weight      ∈ {0.10, 0.25, 0.40}            = 3 варианта
jepa_weight     ∈ {0.50, 1.00}                  = 2 варианта
warmup_steps    ∈ {1500, 3000}                  = 2 варианта
─────────────────────────────────────────────────
Итого: 3×3×2×2 = 36 конфигов
ASHA threshold: ≥3.5 после 4000 шагов
```

---

## 📈 ПРОГРЕСС-ТРЕКЕР

```
START:   3.90 BPB  [████████████████████████████████] Apr 21
Today:   2.519 BPB [████████████████████░░░░░░░░░░░] Apr 25 (-35.4%)
GATE-1:  2.22  BPB [██████████████████░░░░░░░░░░░░░] JEPA Rung-1 target
GATE-2:  2.03  BPB [████████████████░░░░░░░░░░░░░░░] JEPA Rung-2 target
TARGET:  1.50  BPB [████████████░░░░░░░░░░░░░░░░░░░] Apr 30 IGLA
SOTA:    1.22  BPB [██████████░░░░░░░░░░░░░░░░░░░░░] HierAttn GPU

BPB gap to GATE-1:  -0.30
BPB gap to TARGET:  -1.02  (GPU архитектуры + JEPA + Muon + NCA)
```

### Ожидаемая траектория

| Шаг | Техника | Δ BPB | Итого BPB |
|-----|---------|-------|----------|
| baseline | 6-gram ASHA | — | 2.5193 |
| +1 | Attention 1-2L CPU | -0.30 | ~2.22 |
| +2 | T-JEPA (w=0.25) | -0.20 | ~2.02 |
| +3 | Muon NS5 | -0.15 | ~1.87 |
| +4 | NCA regularizer (w=0.25) | -0.15 | ~1.72 |
| +5 | ReLU² | -0.08 | ~1.64 |
| +6 | QK-Gain (4.0) | -0.10 | ~1.54 |
| +7 | GF16 (d_model≥256) | -0.05 | ~1.49 ← **IGLA** |
| +8 | φ-LR schedule | -0.03 | ~1.46 🎯 |

---

## 🔗 LINKS

- **ONE SHOT HUB**: https://github.com/gHashTag/trios/issues/143 🔴 НЕ ЗАКРЫВАТЬ
- Parameter Golf: https://github.com/gHashTag/trios/issues/237
- GF16 Whitepaper: https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md
- JEPA-T Paper: https://github.com/gHashTag/trinity/blob/61bf773204e2fee1379a2598350489f20dd49c83/docs/lab/papers/2026-03-15-hslm-tjepa.md
- arXiv Muon NS5: https://arxiv.org/html/2604.01472v1
- arXiv NCA: https://arxiv.org/abs/2603.10055
- arXiv ReLU²: https://arxiv.org/html/2310.04564v1
- Parameter Golf leaderboard: https://github.com/openai/parameter-golf/issues/83

---

*φ² + 1/φ² = 3 | TRINITY | 2026-04-25 20:00 +07 | All agents: branch=main only*
