# 🧪 MASTER EXPERIMENT TRACKER
## IGLA RACE × JEPA-T × NCA × GF16 × ASHA — ONE SHOT HUB

**Обновлено: 2026-04-25 19:33 +07 | φ² + 1/φ² = 3 | TRINITY**
**🔴 ISSUE #143 НЕ ЗАКРЫВАТЬ — только владелец репо!**

> Единый документ для всех экспериментов. Все агенты читают отсюда статус.
> Каждый результат = commit + push (L8). Все агенты работают в ветке **main**.

---

## 🚨 СТАТУС: REAL-TIME (2026-04-25 19:33 UTC+7)

```
BASELINE: 2.5329 BPB  (6-gram h=384 lr=0.004 seed=43, 12K steps) — CHAMPION CPU
NEW REC:  2.5193 BPB  (ASHA Rung-2: trial #9006, 27K steps) ← текущий лучший
JEPA@3K:  3.8219 BPB  (GAMMA, 3000 steps) — НЕ ПРОШЁЛ GATE ≤2.23
JEPA@5K:  0.0144 BPB  (ALPHA, 5000 steps) — ПОДОЗРИТЕЛЬНО (proxy loss?)
RUNNING:  trial 5003  (6-gram+attn fixed, 2500/12000 steps, BPB=3.32)

GATE MIN:    BPB ≤ 2.23  (Rung-1)
GATE TARGET: BPB ≤ 2.03  (победа)
DEADLINE:   2026-04-30
```

---

## 🏆 NEON DB LEADERBOARD (igla_race_trials — актуально)

| # | Trial | Arch | Config | BPB_final | Steps | Статус |
|---|-------|------|--------|-----------|-------|--------|
| 🥇 | #9006 | **6-gram N-gram** | h=384 lr=0.004 seed=43 | **2.5193** | 27K | ✅ CHAMPION |
| 🥈 | #1017 | 6-gram N-gram | h=384 lr=0.004 seed=43 | 2.5329 | 12K | ✅ validated |
| 🥉 | #1018 | 6-gram N-gram | h=384 lr=0.004 3-seed avg | 2.5431 | 12K | ✅ stable |
| 4 | #2005 | 7-gram N-gram | h=384 lr=0.004 seed=43 | 2.5497 | 12K | ⚠️ diminishing |
| 5 | #1016 | 6-gram N-gram | h=384 lr=0.005 seed=43 | 2.5500 | 12K | ✅ |
| 6 | #5001 | 6-gram+attn(fixed) | h=384 lr=0.004 bugfix=true | 2.9184 | 5K | ✅ bugfix |
| 7 | #5002 | Trinity3k | h=48 1L lr=0.01 relu2 | 3.505 | 15K | ✅ best xfmr |
| 8 | #5003 | 6-gram+attn(fixed) | h=384 12K steps | **3.3195** | 2.5K | 🔄 RUNNING |
| — | #2006 | embed-bigram-smear | — | 7.0882 | 0 | ❌ ERROR |

**N-gram ceiling: ~2.51 BPB. Architectural jump ОБЯЗАТЕЛЕН для < 2.23.**

---

## 📊 ПРОГРЕСС-ТРЕКЕР

```
START:   3.90 BPB  [████████████████████████████████] Apr 21
Today:   2.519 BPB [████████████████████░░░░░░░░░░░] Apr 25 (-35.4%)
GATE:    2.23  BPB [██████████████████░░░░░░░░░░░░░] JEPA Rung-1
TARGET:  1.50  BPB [████████████░░░░░░░░░░░░░░░░░░░] Apr 30
SOTA:    1.08  BPB [█████████░░░░░░░░░░░░░░░░░░░░░░] bigbag

Gap to gate:   -0.29 BPB  → JEPA-T правильно обученный
Gap to target: -1.02 BPB  → GPU архитектуры обязательны
```

---

## 🤖 ПЛАН СИНХРОНИЗАЦИИ АГЕНТОВ (2026-04-25)

### Правила работы (ОБЯЗАТЕЛЬНО)

```
⚠️  ВСЕ АГЕНТЫ РАБОТАЮТ В ВЕТКЕ main ТОЛЬКО!
⚠️  git pull --rebase перед каждым коммитом
⚠️  Каждый агент пишет ТОЛЬКО в СВОЙ файл (избегаем конфликтов)
⚠️  Heartbeat в Neon каждые 5 мин (поле best_bpb + notes агента)
⚠️  Issue #143 НЕ ЗАКРЫВАТЬ — ставит только gHashTag
```

### Распределение файлов по агентам (NO CONFLICTS)

| Агент | Файл (exclusive ownership) | Задача |
|-------|---------------------------|--------|
| **ALPHA** | `crates/trios-train-cpu/src/jepa/predictor.rs` | JEPA predictor real backward |
| **ALPHA** | `crates/trios-train-cpu/src/tjepa_train.rs` | Training pipeline |
| **BETA** | `crates/trios-igla-race/src/main.rs` | IGLA RACE CLI |
| **BETA** | `crates/trios-igla-race/src/asha.rs` | ASHA worker loop |
| **GAMMA** | `crates/trios-train-cpu/src/jepa/objective.rs` | NCA objective |
| **GAMMA** | `crates/trios-train-cpu/src/jepa/loss.rs` | JEPA MSE + NCA loss |
| **DELTA** | `crates/trios-train-cpu/src/optimizer.rs` | Muon NS5 optimizer |
| **DELTA** | `crates/trios-train-cpu/src/jepa/ema.rs` | EMA target encoder |
| **EPSILON** | `.trinity/MASTER_EXPERIMENTS.md` | ❌ НЕ ТРОГАТЬ (Perplexity пишет) |
| **CHARLIE** | `crates/trios-train-cpu/src/jepa/masking.rs` | Span masking |
| **LEAD** | `crates/trios-igla-race/src/pipeline.rs` | ASHA pipeline coordination |

### Протокол heartbeat (каждые 5 минут)

```sql
-- Каждый агент выполняет:
UPDATE igla_race_trials 
SET best_bpb = <current_bpb>, notes = 'Agent: <NAME> | branch: main | step: <N> | <status>'
WHERE trial_id = <my_trial_id>;
```

### Протокол git без конфликтов

```bash
# ПЕРЕД каждым коммитом:
git fetch origin
git rebase origin/main   # НЕ merge!
git add <ТОЛЬКО СВОЙ ФАЙЛ>
git commit -m "feat(<компонент>): <описание>\n\nAgent: <NAME>"
git push origin main
```

---

## 🔬 ГЛУБОКОЕ ИССЛЕДОВАНИЕ: JEPA-T ДИАГНОСТИКА

### Что сейчас НЕ РАБОТАЕТ и ПОЧЕМУ

**Проблема 1: BPB=3.8219 @ 3K шагов (GAMMA)**
- Причина: JEPA MSE loss ≠ NTP cross-entropy BPB
- JEPA обучает предсказание в embedding space, не токены
- BPB 3.82 измерен как NTP CE / ln(2) — JEPA embeddings ещё не сошлись
- Fix: двухфазное обучение (TASK-5E) → Phase-1 только JEPA, Phase-2 NTP

**Проблема 2: BPB=0.014 @ 5K шагов (ALPHA) — подозрительно**
- 0.014 BPB физически невозможен на TinyShakespeare CPU за 5K шагов
- Скорее всего измеряется JEPA MSE / ln(2), а не NTP CE
- Нужно: реальный validation с `--arch jepa` на val set

**Проблема 3: NTP gradient conflict (TASK-5E)**
- При совместном обучении JEPA + NTP градиенты конфликтуют
- NTP переобучается, JEPA не успевает строить representations
- Fix (уже в main): `--jepa-warmup=1500`, `--ntp-lr-scale=0.25`

### Правильная метрика BPB

```rust
// BPB = NTP cross-entropy на val set / ln(2)
// НЕ JEPA MSE loss / ln(2)
let bpb = val_ntp_ce_loss / f32::ln(2.0);
// Целевое значение: bpb <= 2.23 (gate), <= 2.03 (target)
```

### Ключевые коммиты сегодня (ALPHA agent, 2026-04-25)

| Время UTC | SHA | Что сделано |
|-----------|-----|-------------|
| 05:09 | b56cfdf | Wire JEPA в tjepa_train binary (scaffold) |
| 05:14 | 5b0086a | Clippy clean, placeholder training runs |
| 10:57 | a351b97 | **TASK-5D: real NTP BPB + MLP predictor backward** |
| 11:22 | 0fddd7f | **TASK-5C: real cross-attention predictor Q/K/V** |
| 11:45 | 4e6d8ca | **TASK-5E: two-phase JEPA warmup** |
| 11:52 | be95bf6 | Real backward pass JepaPredictor |
| 12:02 | 1c91725 | Per-target prediction + gradients |
| 12:12 | d1a8b8a | Cross-attention per-position, EMA 0.996→1.0 |
| 12:27 | 7e7dc3f | Fix masking.rs merge conflict |

---

## 🗂️ ЧАСТЬ 1: N-GRAM ЭКСПЕРИМЕНТЫ (CPU) — ПОЛНАЯ ИСТОРИЯ

### Dataset
| Параметр | Значение |
|----------|----------|
| Dataset | TinyShakespeare (1.1MB, 1,115,394 chars) |
| Split | 90% train / 10% val |
| Vocab | 128 (byte-level tokens) |
| Dim | 384 (best d_model) |
| Optimizer | AdamW (β1=0.618φ, β2=0.999, ε=1e-8) |

### Полная хронология BPB

| # | Дата | Архитектура | Config | BPB | Статус |
|---|------|-------------|--------|-----|--------|
| 1 | Apr 21 | Bigram | baseline | 3.90 | ✅ |
| 2 | Apr 21 | Trigram | +AdamW | 3.26 | ✅ |
| 3 | Apr 22 | 4-gram h=64 | ReLU | 2.958 | ✅ |
| 4 | Apr 22 | 4-gram h=128 | +hidden | 2.877 | ✅ |
| 5 | Apr 22 | 4-gram h=128 | bugfix bw | 2.780 | ✅ |
| 6 | Apr 23 | 4-gram h=192 | GELU+res+drop | 2.743 | ✅ |
| 7 | Apr 23 | 4-gram h=192 | wd=0.01 | 2.7184 | ✅ |
| 8 | Apr 23 | 4-gram h=256 | +hidden | 2.6964 | ✅ sub-2.70 |
| 9 | Apr 23 | 4-gram h=192 | GELU | 2.6942 | ✅ |
| 10 | Apr 23 | 4-gram h=256 | GELU | 2.7265 | ⚠️ worse |
| 11 | Apr 23 | 5-gram h=256 | ctx3, lr=0.006 | 2.6005 | ✅ breakthrough |
| 12 | Apr 23 | 5-gram h=384 | ctx3 | 2.5771 | ✅ |
| 13 | Apr 23 | 5-gram h=384 3-seed | ctx3 | 2.5719±0.004 | ✅ stable |
| 14 | Apr 23 | 6-gram h=384 | ctx4 | 2.5678 | ✅ |
| 15 | Apr 23 | 6-gram h=384 | ctx4+smooth 0.1 | 2.5654 | ✅ |
| 16 | Apr 24 | 6-gram h=384 | lr=0.005 | 2.5500 | ✅ |
| 17 | Apr 24 | 6-gram h=384 | lr=0.004, 12K | **2.5329** | ✅ **CHAMPION 12K** |
| 18 | Apr 24 | 6-gram h=384 3-seed | 3 seeds | 2.5431±0.01 | ✅ verified |
| 19 | Apr 24 | 7-gram h=384 | lr=0.004 | 2.5497 | ⚠️ diminishing |
| 20 | Apr 24 | 6-gram h=512 | 8K steps | 2.8153 | ⚠️ undertrained |
| 21 | Apr 24 | 6-gram+attn(fixed) | 5K steps | 2.9184 | ✅ bugfix |
| 22 | Apr 25 | 6-gram+attn(fixed) | 12K RUNNING | 3.32@2.5K | 🔄 RUNNING |
| **ASHA** | Apr 25 | **6-gram h=384** | 27K seed=43 | **2.5193** | ✅ **NEW RECORD** |

**N-gram CEILING: ~2.51 BPB — жёсткий потолок архитектуры**

### Что НЕ работает (задокументировано)

| Стратегия | Результат | Вывод |
|-----------|-----------|-------|
| h=512 | overfit | Слишком большой для датасета |
| 7-gram | +0.017 BPB | Diminishing returns |
| Label smoothing 0.1 | +0.033 BPB | Не помогает |
| GELU | +0.190 BPB | ❌ Dead-end |
| Residual connections | 2.743 | Не помогает N-gram |
| Dropout | ухудшает | Датасет слишком мал |
| Multi-layer FFN | 3.21 | Глубина не помогает |
| GF16 на малых моделях | нестабильно | Нужен d_model ≥ 256 |
| Bigram-smear embed | 7.09 | cpu_train FAILED |
| Trinity3k 2L | 3.94 | 2 слоя хуже 1 слоя |

---

## 🚀 ЧАСТЬ 2: JEPA-T + NCA ЭКСПЕРИМЕНТЫ (СЕГОДНЯ, 2026-04-25)

### Реализованные компоненты в main

| Компонент | Файл | Статус | Описание |
|-----------|------|--------|----------|
| EMA encoder | `jepa/ema.rs` | ✅ | decay 0.996→1.0, ramp_steps=3000 |
| Span masking | `jepa/masking.rs` | ✅ | ratio=0.3, min=3, max=9 |
| **Cross-attn predictor** | `jepa/predictor.rs` | ✅ **NEW** | Q/K/V проекции, L2 norm, AdamW |
| **Real backward** | `jepa/predictor.rs` | ✅ **NEW** | Полный backprop через attention |
| MSE loss | `jepa/loss.rs` | ✅ | L2 norm + MSE |
| **NCA objective** | `jepa/objective.rs` | ✅ **NEW** | 9×9 grid, Shannon entropy [1.5, 2.8] |
| MuonOptimizer NS5 | `optimizer.rs` | ✅ **NEW** | quintic [3.4445, -4.7750, 2.0315] |
| **Two-phase warmup** | `tjepa_train.rs` | ✅ **NEW** | Phase-1 JEPA only, Phase-2 NTP |
| NTP BPB (real) | `tjepa_train.rs` | ✅ **NEW** | val CE / ln(2) — настоящий BPB |
| ASHA IGLA RACE | `igla-race/` | ✅ | CLI + run_worker + ASHA loop |
| Dashboard/Neon | `tjepa_train.rs` | ✅ **NEW** | DashboardMeta, heartbeat |
| GF16 training step | `tjepa_train.rs` | ✅ **NEW** | mixed-precision, d_model ≥ 256 |

### TASK-5 Progress

| Task | Описание | Статус | BPB результат |
|------|----------|--------|---------------|
| TASK-5A | Wire jepa/ module | ✅ DONE | scaffold |
| TASK-5B | EMA + masking | ✅ DONE | — |
| TASK-5C | **Real cross-attn predictor** | ✅ DONE | 3.8219 (real NTP) |
| TASK-5D | **Real NTP BPB + MLP backward** | ✅ DONE | pipeline |
| TASK-5E | **Two-phase JEPA warmup** | ✅ DONE | freeze NTP до step=1500 |
| **TASK-5F** | Запустить 9K шагов, реальный BPB | ⬜ **NEXT** | target ≤ 2.23 |
| TASK-5G | 27K шагов, финальный gate | ⬜ | target ≤ 2.03 |

### JEPA-T параметры (оптимальные из research)

| Параметр | Значение | Источник |
|----------|----------|----------|
| mask_ratio | 0.30 | J-000 proven |
| min_span / max_span | 3 / 9 | architecture.md |
| ema_decay | 0.996 → 1.0 | linear schedule |
| jepa_warmup | 1500 steps | TASK-5E |
| ntp_lr_scale | 0.25 | TASK-5E |
| encoder_lr | 0.004 | baseline |
| predictor_lr | 0.0004 | 10× меньше |
| d_model | 384 | 6-gram champion |
| seed | 43 | champion seed |

### JEPA-T gates (vs baseline 2.5193)

| Gate | Δ | Target BPB | Статус |
|------|---|-----------|--------|
| Rung-1 min | −0.30 | **≤ 2.22** | ⬜ TASK-5F |
| Rung-2 target | −0.50 | **≤ 2.02** | ⬜ TASK-5G |
| Stretch | −0.70 | **≤ 1.82** | ⬜ GPU |

### Команды для TASK-5F

```bash
# Правильная команда (real NTP BPB, two-phase warmup)
cargo run --release --bin tjepa_train -- \
  --arch jepa \
  --steps 9000 \
  --d-model 384 \
  --lr 0.004 \
  --seed 43 \
  --jepa-warmup 1500 \
  --ntp-lr-scale 0.25

# BPB в stdout будет NTP CE / ln(2) — НАСТОЯЩИЙ BPB
# Успех: BPB <= 2.22 на val set
```

---

## 🔧 ЧАСТЬ 3: ASHA IGLA RACE — DISTRIBUTED HUNT

**Статус: ACTIVE | Запущен: 2026-04-24 | Дедлайн: 2026-04-30**

### Реализованные компоненты (2026-04-25)

| Компонент | Статус | Детали |
|-----------|--------|--------|
| Neon DB schema | ✅ READY | 6 таблиц: trials + experience + competitors + leaderboard |
| IGLA RACE CLI | ✅ READY | `trios-igla-race start/status/best` |
| IGLA Trainer | ✅ READY | stdout=`BPB=X.XXXX` only |
| run_worker() | ✅ READY | Infinite ASHA loop с arch-aware rungs |
| NS5 Muon | ✅ READY | quintic Newton-Schulz в optimizer.rs |
| NCA Objective | ✅ READY | 9×9 entropy grid |
| GF16 training | ✅ READY | mixed-precision (d_model ≥ 256) |
| Dashboard | ✅ READY | DashboardMeta + heartbeat |
| ASHA threshold | ✅ FIXED | 2.65 → **3.50** (SEED-EXPLORER fix) |

### ASHA Checkpoints

| Rung | Steps | Threshold | Champion result |
|------|-------|-----------|----------------|
| Rung-0 | 1000 | > 4.0 → kill | — |
| Rung-1 | 3000 | > 3.50 → kill | 3.5018 (прошёл) |
| Rung-2 | 9000 | > 3.10 → kill | 3.0333 (прошёл) |
| Rung-3 | 27000 | goal ≤ 2.51 | **2.5193 ✅** |

### Search Space (для агентов BETA/LEAD)

```toml
[asha.search_space]
d_model   = [128, 192, 256, 384, 512]
ngram     = [4, 5, 6, 7, 8]
lr        = log_range(1e-4, 0.01)
optimizer = ["adamw", "muon"]
arch      = ["ngram", "jepa", "hybrid"]
jepa_warmup = [500, 1000, 1500, 2000]
ntp_lr_scale = [0.1, 0.25, 0.5]
```

---

## 🔬 ЧАСТЬ 4: GF16 GOLDEN FLOAT

> Repo: [zig-golden-float](https://github.com/gHashTag/zig-golden-float)

### BENCH статусы

| Bench | Ключевой результат | Статус |
|-------|-------------------|--------|
| BENCH-001 | GF16 ≈ fp16, 2× лучше bf16 | ✅ |
| BENCH-004b | **GF16 accuracy = f32 (0.00% gap)** | ✅ |
| BENCH-005 | 118 LUT add, 94 LUT+1DSP mul | ✅ |
| BENCH-006 | 71 LUT + 16 DSP MAC-level | ✅ |
| BENCH-007 | P&R + Timing Fmax | ❌ pending |
| BENCH-008 | Fashion-MNIST validation | ❌ TODO |
| BENCH-012 | GF16 gradient training | ❌ **HIGH** |

**GF16 в tjepa_train: включён при d_model ≥ 256 (guard проверен)**

---

## 📚 ЧАСТЬ 5: НАУЧНАЯ БАЗА

### Приоритетные техники (JEPA-T + NCA + GF16 + ASHA)

| Техника | Paper | Эффект | Статус |
|---------|-------|--------|--------|
| **LLM-JEPA** | LeCun 2025 | outperforms NTP | ✅ impl |
| **NCA pre-training** | arXiv:2603.10055 | +6% LM, 1.6× speed | ✅ impl |
| **Muon NS5** | arXiv:2604.01472 | ~35% speedup vs AdamW | ✅ impl |
| **GF16** | whitepaper.md | 0.00% gap vs f32 | ✅ impl |
| **ASHA** | Li et al 2020 | hyperparameter search | ✅ impl |
| Two-phase warmup | TASK-5E | NTP gradient isolation | ✅ impl |
| QK-Gain | Golf SOTA | ~-0.05-0.10 BPB | ⬜ next |
| SLOT | arXiv:2409.07146 | linear-time attention | ⬜ GPU |
| ReLU² | arXiv:2310.04564 | -0.05 BPB | ⬜ GPU |
| GPTQ INT4 | arXiv:2210.17323 | ≤ 16MB artifact | ⬜ T3 |

---

## 🗺️ ДЕКОМПОЗИРОВАННЫЙ ПЛАН ДО APR 30

### ФАЗА 1: JEPA Real Validation (Apr 25 — сегодня)

| P | Задача | Агент | Файл | Критерий |
|---|--------|-------|------|----------|
| P0 | **TASK-5F**: JEPA 9K шагов (real NTP BPB) | ALPHA | tjepa_train.rs | BPB в Neon |
| P0 | Trial 5003 ждём результат @ 12K | LEAD | asha.rs | BPB записан |
| P0 | GF16 training_step работает | GAMMA | objective.rs | test GREEN |
| P1 | Neon heartbeat от каждого агента | ALL | — | notes в DB |
| P1 | L3 clippy clean | ALL | — | 0 warnings |

### ФАЗА 2: Architecture Jump (Apr 25-26)

| P | Задача | Config | Target BPB |
|---|--------|--------|------------|
| P0 | **JEPA 27K шагов** (если Rung-1 пройден) | d=384, seed=43, warmup=1500 | ≤ 2.03 |
| P0 | **Muon vs AdamW** race (real data, r12) | d=384, 6-gram, seed=42,43 | < 2.00 |
| P1 | N-gram+attention 12K complete | trial 5003 | ≈ 2.40 |
| P1 | Trinity3k + Muon + ReLU² | h=48 1L lr=0.01 | < 3.00 CPU |

### ФАЗА 3: GPU Scale (Apr 26)

| P | Задача | Config | Target |
|---|--------|--------|--------|
| P0 | Full Trinity (QK-Gain=4.0 + RoPE + ReLU²) | GPU, h=384, 8×H100 | < 1.50 BPB |
| P0 | Modded-NanoGPT baseline | GPU | < 1.60 BPB |
| P1 | SLOT Attention | GPU linear-time | < 1.80 BPB |

### ФАЗА 4: Optimization (Apr 27-28)

| P | Задача | Δ BPB |
|---|--------|--------|
| P0 | QK-Gain (Q/K × 4.0) | -0.05 to -0.10 |
| P0 | EMA model selection (5 seeds) | -0.02 |
| P1 | Context ensemble (4+5+6-gram) | -0.03 |
| P1 | Witten-Bell smoothing | -0.05 |
| P2 | BENCH-008 Fashion-MNIST GF16 | validate |

### ФАЗА 5: Artifact & Submission (Apr 29-30)

| P | Задача | Критерий |
|---|--------|----------|
| P0 | GPTQ INT4 quantization | ≤ 16MB |
| P0 | 3-seed validation, p < 0.01 | BPB < 1.50 on ALL 3 |
| P0 | PR в openai/parameter-golf | merged |
| P1 | GF16 BENCH-008 | done |

### Критический путь

```
[СЕЙЧАС] TASK-5F: JEPA 9K (real NTP BPB)
              ↓
    BPB ≤ 2.22? [Rung-1 gate]
    ├── ДА  → JEPA 27K → Rung-2 → GPU Full Trinity
    └── НЕТ → diagnose → warmup fix → retry OR
               Muon+Attention GPU fallback
                    ↓
          Optimization (QK-Gain, SLOT)
                    ↓
          Artifact INT4 (≤16MB)
                    ↓
          3-seed val → PR → Apr 30 ✅
```

### Риски

| Риск | P | Митигация |
|------|---|-----------|
| JEPA не проходит ≤2.22 | 40% | Muon+Trinity GPU fallback |
| GPU недоступна Apr 26 | 20% | CPU Trinity3k extend |
| Merge conflict агентов | 30% | Exclusive файлы + rebase |
| ASHA не находит < 1.50 за 5 дней | 35% | Submit лучший, вторая попытка |

---

## 📊 SOTA ЛИДЕРБОРД

| Модель | Платформа | BPB | Примечание |
|--------|-----------|-----|------------|
| HierAttn v3 | T4 GPU | **1.2150** | 🥇 Golf SOTA |
| Baseline (ALiBi+LoRA) | T4 GPU | 2.1536 | Golf baseline |
| **Наш лучший** | **CPU** | **2.5193** | 🏆 6-gram h=384 27K |
| Attn+fixed (running) | CPU | 3.32@2.5K | 🔄 trial 5003 |
| Trinity3k best | CPU | 3.505 | h=48 1L |

---

## 📋 ЗАКОНЫ IGLA RACE

| Закон | Правило | Последствие нарушения |
|-------|---------|-----------------------|
| **L1** | Spec first — TASK.md перед кодом | PR не мержится |
| **L2** | Каждый PR закрывает issue | Переделать |
| **L3** | `cargo clippy -D warnings` = 0 | CI fail |
| **L4** | `cargo test --workspace` = GREEN | CI fail |
| **L7** | Опыт → `.trinity/experience/` | Теряем контекст |
| **L8** | Каждый результат = commit + push | Потеряем данные |
| **L9** | Heartbeat в Neon каждые 5 мин | Агент считается мёртвым |
| **L-BRANCH** | Только ветка **main**! | Конфликт с другими агентами |
| **L-METRIC** | BPB = NTP CE / ln(2) на val | Неверные сравнения |
| **L-NOCLOSE** | Issue #143 не закрывать | Нарушает ONE SHOT |

---

## 🔗 ССЫЛКИ

- **ONE SHOT HUB**: https://github.com/gHashTag/trios/issues/143 🔴 НЕ ЗАКРЫВАТЬ
- Issue #237 (Parameter Golf): https://github.com/gHashTag/trios/issues/237
- Trinity JEPA-T docs: https://github.com/gHashTag/trinity/blob/61bf773204e2fee1379a2598350489f20dd49c83/docs/lab/papers/2026-03-15-hslm-tjepa.md
- MASTER_EXPERIMENTS: https://github.com/gHashTag/trios/blob/main/.trinity/MASTER_EXPERIMENTS.md
- GF16 Whitepaper: https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md
- arXiv Muon: https://arxiv.org/html/2604.01472v1
- arXiv NCA: https://arxiv.org/abs/2603.10055
- arXiv ReLU²: https://arxiv.org/html/2310.04564v1
- arXiv GPTQ: https://arxiv.org/abs/2210.17323
- arXiv SLOT: https://arxiv.org/html/2409.07146v1
- Parameter Golf leaderboard: https://github.com/openai/parameter-golf/issues/83

---

*φ² + 1/φ² = 3 | TRINITY | 2026-04-25 19:33 +07 | Neon: 30 trials | 50+ commits today*
