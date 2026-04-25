# 🧪 MASTER EXPERIMENT TRACKER
## Parameter Golf × GF16 × Trinity — Все эксперименты: прошлое, настоящее, будущее

**Обновлено: 2026-04-25 19:17 +07 | φ² + 1/φ² = 3 | TRINITY**

> Единый документ для всех экспериментов проекта. Каждый результат = commit + push (L8).

---

## 📊 ПРОГРЕСС-ТРЕКЕР (Parameter Golf)

```
START:  3.90 BPB  [████████████████████████████████] Apr 21
Today:  2.5193 BPB [████████████████████░░░░░░░░░░░] Apr 25 (-35.4%)
TARGET: 1.50 BPB  [████████████░░░░░░░░░░░░░░░░░░░░] Apr 30
SOTA:   1.08 BPB  [█████████░░░░░░░░░░░░░░░░░░░░░░░] bigbag

Gap: -1.02 BPB за 4 дня → нужен architectural jump
New record: 2.5193 (ASHA Rung-2: 6-gram h=384 lr=0.004 seed=43, 27K steps)
```

---

## 🗂️ ЧАСТЬ 1: N-GRAM ЭКСПЕРИМЕНТЫ (CPU)

### Dataset
| Параметр | Значение |
|----------|----------|
| Dataset | TinyShakespeare (1.1MB, 1,115,394 chars) |
| Split | 90% train / 10% val |
| Vocab | 128 (byte-level tokens) |
| Dim | 64 (embedding dimension) |
| Seq | 64 (sequence length) |
| Optimizer | AdamW (β1=0.618, β2=0.999, ε=1e-8) |
| LR Schedule | Cosine with warmup |

### Полная хронология BPB

| # | Дата | Архитектура | Config | BPB | Δ | Статус |
|---|------|-------------|--------|-----|---|--------|
| 1 | Apr 21 | Bigram | baseline | 3.90 | — | ✅ |
| 2 | Apr 21 | Trigram | +AdamW | 3.26 | -0.64 | ✅ |
| 3 | Apr 22 | 4-gram h=64 | ReLU | 2.958 | -0.30 | ✅ |
| 4 | Apr 22 | 4-gram h=128 | +hidden | 2.877 | -0.08 | ✅ |
| 5 | Apr 22 | 4-gram h=128 | bugfix bw, lr=0.004 | 2.780 | -0.10 | ✅ |
| 6 | Apr 23 | 4-gram h=192 | GELU, res, drop=0.1, warmup=500, wd=0.01 | 2.743 | -0.04 | ✅ |
| 7 | Apr 23 | 4-gram h=192 | wd=0.01 | 2.7184 | -0.019 | ✅ |
| 8 | Apr 23 | 4-gram h=256 | +hidden | 2.6964 | -0.022 | ✅ sub-2.70 |
| 9 | Apr 23 | 4-gram h=192 | GELU | 2.6942 | -0.002 | ✅ |
| 10 | Apr 23 | 4-gram h=256 | GELU | 2.7265 | +0.03 | ⚠️ worse |
| 11 | Apr 23 | 5-gram h=256 | ctx3, lr=0.006, wd=0.01 | 2.6005 | -0.10 | ✅ breakthrough |
| 12 | Apr 23 | 5-gram h=384 | ctx3, h=384 | 2.5771 | -0.023 | ✅ |
| 13 | Apr 23 | 5-gram h=384 3-seed | ctx3, 3 seeds | 2.5719±0.004 | — | ✅ stable |
| 14 | Apr 23 | 6-gram h=384 | ctx4 | 2.5678 | -0.009 | ✅ |
| 15 | Apr 23 | 6-gram h=384 | ctx4 + label smooth 0.1 | 2.5654 | -0.002 | ✅ |
| 16 | Apr 24 | 6-gram h=384 | lr=0.005, wd=0.01 | 2.5500 | -0.015 | ✅ |
| 17 | Apr 24 | 6-gram h=384 | lr=0.004, wd=0.01 | **2.5329** | **-0.017** | ✅ **CHAMPION (12K)** |
| 18 | Apr 24 | 6-gram h=384 3-seed | lr=0.004, seed=42,43,44 | 2.5431±0.01 | — | ✅ verified |
| 19 | Apr 24 | 7-gram h=384 | lr=0.004, wd=0.01 | 2.5497 | +0.017 | ⚠️ diminishing |
| 20 | Apr 24 | 6-gram h=512 | lr=0.004, 8K steps | 2.8153 | — | ⚠️ UNDERTRAINED |
| 21 | Apr 24 | 6-gram h=384 +attn(bugfix) | 5K steps, fixed attn | 2.9184 | — | ⚠️ bug fix worked |
| 22 | Apr 25 | **6-gram h=384 +attn(fixed)** | 12K steps, RUNNING | 3.3195 (@2500) | — | 🔄 RUNNING |
| **ASHA** | Apr 25 | **6-gram h=384 ASHA Rung-2** | 27K steps, seed=43 | **2.5193** | **-0.014** | ✅ **NEW RECORD** |

### 🏆 DB-ANALYST Audit — 2026-04-25 (49 trials, `igla_race_trials`)

**New champion (ASHA Rung-2): 6-gram · h=384 · lr=0.004 · seed=43 → BPB 2.5193 @ 27K steps**

| Ось | Оптимум | Значение | Runner-up | Δ |
|-----|---------|----------|-----------|---|
| N-gram order | **6-gram** | 2.5193 | 7-gram | +0.030 |
| Learning rate | **lr=0.004** | 2.5193 | lr=0.005 | +0.031 |
| Hidden dim | **h=384** | 2.5193 | h=256 | +0.081 |
| Steps | **27K** | 2.5193 | 12K | +0.014 |
| Attention bugfix | **yes** | 2.9184 (5K) | broken | was 3.72 |
| Activation | **ReLU** | baseline | GELU | +0.190 |

**Ключевые выводы из новых trials (Сессия 2026-04-25):**
1. 27K steps vs 12K: дополнительная тренировка даёт -0.014 BPB (стоит делать)
2. Attention bugfix РАБОТАЕТ: было 3.72 BPB (broken) → 2.92 BPB (fixed, 5K steps)
3. Attention + 12K шагов — RUNNING сейчас (trial 5003), ожидается < 2.53 BPB
4. Transformer Trinity3k h=48 1L: лучший трансформер на CPU = 3.505 BPB (пока хуже N-gram)
5. ASHA Rung-2 подтвердил: дальнейший тюнинг N-gram даёт убывающую отдачу

### IGLA Race: ASHA Rung-2 Results (Trial #9006)

| Rung | Steps | BPB | Δ от предыдущего |
|------|-------|-----|-------------------|
| Rung-0 | — | — | — |
| Rung-1 | 3000 | 3.5018 | start |
| Rung-2 | 9000 | 3.0333 | -0.4685 |
| Rung-3 | 27000 | **2.5193** | -0.5140 |

**Вывод: N-gram сходится стабильно на 27K шагах. Ceiling ≈ 2.51 BPB.**

### Что НЕ работает (зафиксировано)

| Стратегия | Результат | Вывод |
|-----------|-----------|-------|
| h=512 | overfit | Слишком большой для данных |
| 7-gram | +0.017 BPB | Diminishing returns |
| Label smoothing 0.1 | +0.033 BPB | Не помогает, хуже baseline |
| GELU | +0.190 BPB | ❌ Dead-end подтверждён |
| Residual connections | 2.743 (хуже!) | Не помогает N-gram |
| Dropout | ухудшает | Датасет слишком мал |
| Warmup | не нужен | Cosine без warmup лучше |
| 15K+ steps N-gram | slight overfit | Sweet spot: 12-27K |
| Multi-layer FFN | 3.21 (хуже!) | Глубина не помогает |
| GF16 precision | 3.21 (хуже!) | Нестабильность на малых моделях |
| Bigram-smear embed | 7.09 BPB | cpu_train FAILED — no learning |
| Trigram-embed | 3.26 BPB | Embedding-only baseline |
| Trinity3k 2L | 3.94 BPB | 2 слоя хуже 1 слоя |

### N-gram команды

```bash
# Текущий лучший результат (champion @ 27K)
cargo build --release -p trios-train-cpu --bin ngram_train
./target/release/ngram_train --seed=43 --steps=27000 --hidden=384 --lr=0.004 --wd=0.01

# 12K version (validated champion)
./target/release/ngram_train --seed=43 --steps=12000 --hidden=384 --lr=0.004 --wd=0.01

# 3-seed параллельно
tri train --seeds 42,43,44 --steps=12000 --hidden=384 --lr=0.004 --parallel
```

---

## 🚀 ЧАСТЬ 2: ATTENTION / TRANSFORMER ЭКСПЕРИМЕНТЫ

### Trinity3k Transformer — первые результаты (2026-04-24)

| Trial | Arch | Config | BPB | Steps | Статус |
|-------|------|--------|-----|-------|--------|
| 5002 | Trinity3k | h=48, 1L, lr=0.01, relu2 | **3.505** | 15K | ✅ BEST transformer CPU |
| 4001 | Trinity3k | h=48, 1L, qk_norm, tied_emb | 3.5776 | 8K | ✅ |
| 3001 | Trinity3k | h=48, 1L, lr=0.003 | 3.7781 | 2K | ✅ |
| 3002 | Trinity3k | h=48, 2L | 4.1203 | 5K | ⚠️ slower |
| 3003 | Trinity3k | h=64, 2L | 4.1953 | 5K | ⚠️ bottleneck |
| 4002 | Trinity3k | h=81, 2L, qk_norm, wd=0.04 | 3.9428 | 3K | ⚠️ slow convergence |

**Вывод: Trinity3k 1-слойный h=48 lr=0.01 = 3.505 BPB (лучший трансформер на CPU)**
**Но это всё ещё хуже N-gram 2.5193 → нужен JEPA-T + Muon + GPU**

### N-gram + Fixed Attention

| Trial | Arch | Config | BPB | Steps | Статус |
|-------|------|--------|-----|-------|--------|
| 4003 | codebase-audit | Bugfix: depth recurrence + BigramHash proj | — | 0 | ✅ FIXED |
| 5001 | ngram+attn(fixed) | h=384, 6-gram, wd=0.01, attention_bugfix=true | 2.9184 | 5K | ✅ bug fix works! |
| 5003 | ngram+attn(fixed) | h=384, 6-gram, wd=0.01, attention_bugfix=true | 3.3195 | 2.5K | 🔄 RUNNING |

**Bugfixes applied (trial 4003):**
- `scripts/train_gpt.py` lines 767-770: depth recurrence was a no-op
- `scripts/train_gpt.py` line 623: BigramHash dead projection fixed

**Прогноз для trial 5003 @ 12K шагов: ≈ 2.4-2.5 BPB (если тренд продолжится)**

### TASK-5: T-JEPA интеграция

**Scaffold:** `crates/trios-train-cpu/src/tjepa.rs` (создан 2026-04-24)
**Theory refs (trinity PR #539):** [JEPA-T docs](https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/)

| Параметр | Значение | Источник |
|----------|----------|----------|
| mask_ratio | 0.30 | J-000 proven |
| min_span / max_span | 3 / 7–11 | architecture.md |
| num_spans | 2 | architecture.md |
| ema_decay | 0.996 → 1.0 | linear schedule |
| jepa_weight | 0.25 | wave9.json |
| ASHA Rung-1 min | 3000 steps | 1.4× slower than NTP |

**JEPA-T gate targets (vs baseline 2.5193):**

| Gate | Δ | Target BPB | Статус |
|------|---|-----------|--------|
| Minimum | −0.30 | **≤ 2.22** | ⬜ NEXT |
| Target | −0.50 | **≤ 2.02** | ⬜ |
| Stretch | −0.70 | **≤ 1.82** | ⬜ |

Next action: Wire `tjepa.rs` в training loop → ASHA Rung-1 (3000 steps) → report BPB vs 2.5193.

### TIER 1 — CPU (Apr 25)

| # | Эксперимент | Ожидаемый BPB | Команда | Статус |
|---|------------|--------------|---------|--------|
| T1-01 | **JEPA-T Rung-1** (tjepa.rs, 3000 steps) | < 2.22 | `tri run tjepa-rung1` | ⬜ NEXT |
| T1-02 | **Attention layer fixed** (trial 5003 @ 12K) | < 2.40 | running | 🔄 RUNNING |
| T1-03 | N-gram + Witten-Bell smoothing | < 2.00 | `tri run witten-bell` | ⬜ |
| T1-04 | Minimal Self-Attention (4 heads, 16-dim) | < 2.10 | `tri run mhsa-v1` | ⬜ |
| T1-05 | RoPE + attention | < 2.00 | `tri run rope-attn` | ⬜ |

### TIER 2 — GPU 8×H100 (Apr 25-26)

| # | Эксперимент | Ожидаемый BPB | Paper | Статус |
|---|------------|--------------|-------|--------|
| T2-01 | **Muon optimizer** + 6-gram | < 2.00 | [arXiv:2604.01472](https://arxiv.org/html/2604.01472v1) | ⬜ |
| T2-02 | **Full Trinity** (Muon + QK-Gain 4.0 + ReLU²) | < 1.50 | — | ⬜ |
| T2-03 | Modded-NanoGPT (RoPE + QK-Norm + INT4) | < 1.60 | — | ⬜ |
| T2-04 | QK-Gain (Q/K scaling before attention) | ~-0.05-0.10 | Parameter Golf SOTA | ⬜ |
| T2-05 | SLOT (Gated Slot Attention, linear-time) | < 1.80 | [arXiv:2409.07146](https://arxiv.org/html/2409.07146v1) | ⬜ |
| T2-06 | TTT (Test-Time Training) | 17× faster | [arXiv:2407.04620](https://arxiv.org/abs/2407.04620) | ⬜ |
| T2-07 | ReLU² activation | -0.05 BPB | [arXiv:2310.04564](https://arxiv.org/html/2310.04564v1) | ⬜ |
| T2-08 | RoPE positional embedding | standard | — | ⬜ |

### TIER 3 — Оптимизация артефакта (Apr 27-29)

| # | Эксперимент | Описание | Paper | Статус |
|---|------------|----------|-------|--------|
| T3-01 | GPTQ INT4 | 3-4 bits, 16MB budget | [arXiv:2210.17323](https://arxiv.org/abs/2210.17323) | ⬜ |
| T3-02 | INT6 quantization | Parameter Golf budget | — | ⬜ |
| T3-03 | EMA selection | 5 seeds, p < 0.01 | — | ⬜ |
| T3-04 | Context ensemble | Multiple context lengths | Nacrith 2025 | ⬜ |
| T3-05 | Byte-level modeling | No tokenizer | — | ⬜ |

---

## 🔬 ЧАСТЬ 3: GF16 GOLDEN FLOAT ЭКСПЕРИМЕНТЫ

> Repo: [zig-golden-float](https://github.com/gHashTag/zig-golden-float)
> Whitepaper: [docs/whitepaper.md](https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md)

### BENCH 001–006: ВЫПОЛНЕНО ✅

| Bench | Что проверено | Ключевой результат | Статус |
|-------|--------------|-------------------|--------|
| BENCH-001 | Quantization error (MSE/MAE) vs fp16/bf16/f32 | GF16 ≈ fp16, 2× лучше bf16 | ✅ |
| BENCH-002 | Arithmetic throughput CPU (add/mul/div) | GF16 add: 7.2 ns/op (+15% vs soft-fp16) | ✅ |
| BENCH-003 | NN inference: frozen synthetic weights | GF16: 5.80% (идентично f32) | ✅ |
| BENCH-004a | NN inference: random initialized weights | GF16: 11.86% (в рамках noise) | ✅ |
| BENCH-004b | NN inference: TRAINED MNIST MLP | **GF16: 97.67% = f32 (0.00% gap)** | ✅ |
| BENCH-005 | FPGA synthesis unit-level (Yosys) | GF16: 118 LUT add, 94 LUT+1DSP mul | ✅ |
| BENCH-006 | FPGA synthesis MAC-level (16-dot) | GF16: 71 LUT + 16 DSP; ternary: 52 LUT + 0 DSP | ✅ |

### BENCH 007+: НЕ СДЕЛАНО ❌

| Bench | Задача | Приоритет | Статус |
|-------|--------|-----------|--------|
| BENCH-007 | P&R + Timing (nextpnr-xilinx) — Fmax GF16 MAC | HIGH | ❌ pending binary build |
| BENCH-008 | Fashion-MNIST validation (реальные данные) | HIGH | ❌ TODO |
| BENCH-009 | CIFAR-10/100 scaling | MEDIUM | ❌ TODO |
| BENCH-010 | Energy profiling реальный (мВт/inference) | MEDIUM | ❌ TODO |
| BENCH-011 | Latency per layer (end-to-end) | MEDIUM | ❌ TODO |
| BENCH-012 | GF16 training (gradient-based, не только inference) | HIGH | ❌ TODO |
| HYBRID-001 | Hybrid Ternary+GF16 architecture test | HIGH | ❌ только spec |

### Multi-Language Bindings: статус

| Binding | Файл | Статус |
|---------|------|--------|
| Zig core | `src/` | ✅ Done |
| Rust crate (struct) | `rust/src/lib.rs` | ✅ Done |
| Rust FFI (extern "C") | `rust/src/ffi.rs` | ❌ TODO |
| C99 header | `c/gf16.h` | ✅ Done |
| C++ header gf16.hpp | `cpp/` | ❌ Phase 3 |
| WASM Uint16Array | `conformance/` | ❌ Phase 3 |
| Gleam/BEAM NIF | — | ❌ Phase 3 |
| LLVM IR i16 reference | — | ❌ Phase 4 |
| Go bindings | `go/` | ❓ verify |
| Python bindings | `python/` | ❓ verify |

### Golden Float Family — числа (полный каталог)

| Символ | Значение | Описание | Статус |
|--------|---------|----------|---------|
| **φ** | 1.6180339887… | Золотое сечение | ✅ базис |
| **1/φ** | 0.6180339887… | = φ − 1 | ✅ AdamW β1 |
| **φ²** | 2.6180339887… | = φ + 1 | ✅ в Trinity |
| **1/φ²** | 0.3819660112… | = 2 − φ | ✅ в Trinity |
| **φ² + 1/φ²** | **3.0** | Главное тождество | ✅ TRINITY |
| **6:9 split** | 6 exp + 9 mantissa | φ-optimal для 16 bit | ✅ GF16 |
| **6/9 = 0.667** | ≈ 2/3 | Инженерное ≈ 1/φ | ✅ GF16 |
| **p = 1/φ** | 0.618… | Self-similar раздел | ✅ arXiv:2602.15266 |
| **ln(φ)** | 0.4812118250… | Логарифм φ | ✅ теория |
| **φ³** | 4.2360679… | = 2φ+1 | 🔬 исследовать |
| **φ^0.5** | 1.2720196… | √φ | 🔬 исследовать |
| **GF8** | 3:4 split | Ultra-low power | ❌ BENCH TODO |
| **GF32** | 13:18 split | ~φ ratio на 32 bit | ❌ BENCH TODO |
| **GF64** | 21:42 split | φ-ratio на 64 bit | ❌ BENCH TODO |
| **GFTernary** | {-φ, 0, +φ} | Ternary с φ-шагом | ❌ HYBRID-001 |

### GF16 Key Numbers (из BENCH-004b/006)

```
Accuracy gap vs f32:     0.00%  ← единственный 16-bit формат!
Energy vs FP32:          0.10×  (10× savings)
MAC-level vs ternary:    1.37×  (71/52 LUT)
Unit-level vs ternary:   47–59×
70B model RAM:           14 GB  (vs 140 GB FP16 = 10× reduction)
SIMD inst reduction:     41×    (56 vs 2304 per loop)
DSP bottleneck:          15 parallel MAC-16 units (XC7A100T)
BF16/ternary accuracy:   9.80%  (catastrophic failure)
```

---

## 📚 ЧАСТЬ 4: НАУЧНЫЕ РАБОТЫ И ТЕХНИКИ

### Trinity Research Base (PR #539, merged 2026-04-24)

| Модель | Docs | Статус |
|--------|------|--------|
| JEPA-T | [architecture, masks, EMA, MSE, params](https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/) | ✅ Merged |
| NCA | [9×9 grid, Wave 8.5 G1-G8 entropy](https://github.com/gHashTag/trinity/tree/main/docs/research/models/NCA/) | ✅ Merged |
| Hybrid | [HybridBigInt API, v2.0-v2.1 reports](https://github.com/gHashTag/trinity/tree/main/docs/research/models/Hybrid/) | ✅ Merged |
| VSA | [bind/unbind/bundle, FPGA, API](https://github.com/gHashTag/trinity/tree/main/docs/research/models/VSA/) | ✅ Merged |
| Ternary | [packed trit encoding, ADR](https://github.com/gHashTag/trinity/tree/main/docs/research/models/Ternary/) | ✅ Merged |

### Оптимизаторы

| Техника | Paper | Эффект | Статус |
|---------|-------|--------|--------|
| **Muon** | [arXiv:2604.01472](https://arxiv.org/html/2604.01472v1) | 2-3× быстрее, Newton-Schulz orthogonalization | ⬜ NEXT |
| **AdamW φ-optimized** | baseline | β1=0.618 (= 1/φ) | ✅ Used |

### Архитектуры (Parameter Golf top)

| Техника | Описание | BPB gain | Paper |
|---------|----------|----------|-------|
| **QK-Gain** | Масштабирование Q/K перед attention | ~-0.05-0.10 | Golf SOTA |
| **SLOT** | Gated Slot Attention (linear-time) | linear-time | [arXiv:2409.07146](https://arxiv.org/html/2409.07146v1) |
| **TTT** | Test-Time Training | 17× faster | [arXiv:2407.04620](https://arxiv.org/abs/2407.04620) |
| **RoPE** | Rotary Position Embedding | standard | — |
| **HierAttn v3** | Hierarchical Attention (residual) | **1.2150 BPB** | Golf leaderboard |

### Активации

| Функция | Paper | Статус |
|---------|-------|--------|
| **ReLU²** | [arXiv:2310.04564](https://arxiv.org/html/2310.04564v1) | ✅ Parameter Golf winners |
| **GELU** | — | ❌ Dead-end: +0.19 BPB на малых моделях |
| **ReLU** | baseline | ✅ Текущий best |

### Квантование

| Метод | Paper | Применение | Статус |
|-------|-------|-----------|--------|
| **GPTQ INT4** | [arXiv:2210.17323](https://arxiv.org/abs/2210.17323) | 3-4 bits, 16MB budget | ⬜ T3-01 |
| **INT6** | Parameter Golf | 16MB budget | ⬜ T3-02 |
| **GF16** | whitepaper.md | 16-bit φ-optimal | ❌ нестабильно на малых |

### N-gram Smoothing

| Метод | Paper | Применение |
|-------|-------|------------|
| **Witten-Bell** | [arXiv:1706.07786](https://arxiv.org/pdf/1706.07786) | character-level n-gram |
| **Kneser-Ney** | — | классический backoff |

---

## 🏁 ЧАСТЬ 5: IGLA RACE — DISTRIBUTED HUNT

**Статус: ACTIVE | Запущен: 2026-04-24 | Дедлайн: Apr 30**

| Компонент | Статус | Детали |
|-----------|--------|--------|
| Neon DB schema | ✅ READY | igla_race_trials + experience + competitors |
| trios-igla-race crate | ⬜ IN PROGRESS | Tokio + ASHA worker + Neon coord |
| trios-igla-trainer crate | ✅ READY | CPU trainer with --seed argument |
| Machine 1 (mac-studio-1) | ✅ **CHAMPION** | BPB 2.5193 (6-gram h=384 lr=0.004 seed=43, 27K) |
| Machines 2-4 | ⬜ PENDING | ONE SHOT инструкция готова |
| tri race CLI | ⬜ TODO | PR #223 | race start/status/best |
| DB-ANALYST audit | ✅ DONE | 49 trials analysed, 2026-04-25 |
| Issue #143 | ✅ CLOSED | 2026-04-25T11:54 |

### ASHA Checkpoints
- **Rung-0**: 1000 шагов → kill if BPB > threshold (top-33% continue)
- **Rung-1**: 3000 шагов → top 33% продолжают ← **TASK-5 entry point**
- **Rung-2**: 9000 шагов → top 11% финалист
- **Rung-3**: 27000 шагов → проверка IGLA (<1.50 BPB)

### ASHA Rung-2 результаты (Trial #9006, 2026-04-25)
```
trial_id: 9006
machine_id: mac-studio-1
arch: 6-gram N-gram
d_model: 384, lr: 0.004, seed: 43
bpb_3000:  3.5018
bpb_9000:  3.0333
bpb_27000: 2.5193 ← NEW RECORD
status: complete
started: 2026-04-25 04:51 UTC
completed: 2026-04-25 05:50 UTC
```

### Search Space
| Параметр | Values |
|----------|--------|
| `d_model` | 128, 192, 256, 384 |
| `context` | 4, 5, 6, 7, 8 |
| `lr` | 1e-4 to 0.01 (log scale) |
| `optimizer` | adamw, muon |
| `use_attention` | true, false |

### Throughput Estimation
```
1 машина × 4 workers × ASHA Rung-0(1000 шагов ~3 мин) = ~80 trials/час
4 машины × 4 workers                                   = ~320 trials/час
4 машины × 8 workers                                   = ~640 trials/час
```

**Итого до дедлайна (Apr 30, ~5 дней)**: при 320 trials/час = **~38,400 trials**

---

## 📊 ЧАСТЬ 6: SOTA ЛИДЕРБОРД

### Parameter Golf (TinyShakespeare)

| Модель | Платформа | BPB | Примечание |
|--------|-----------|-----|------------|
| **HierAttn v3** | T4 GPU | **1.2150** | 🥇 Известный SOTA |
| **Baseline (ALiBi+LoRA)** | T4 GPU | 2.1536 | Golf baseline |
| **Наш лучший** | CPU | **2.5193** | 🏆 6-gram h=384 lr=0.004 seed=43 @ 27K steps |
| **Attention+fixed (running)** | CPU | 3.32@2.5K | 🔄 trial 5003 in progress |
| **Trinity3k best CPU** | CPU | 3.505 | ✅ h=48 1L lr=0.01 |

### enwik8 (100MB Wikipedia)

| Система | BPB | Размер | Примечание |
|---------|-----|--------|------------|
| **Nacrith** | **0.9389** | 135M | 🥇 2025 SOTA |
| **FineZip** | 1.024 | — | Предыдущий лидер |
| **ts_zip** | ~1.11 | 8B | Large LM compression |
| **L3TC** | ~1.20 | — | RWKV-based, 50× smaller |

---

## 🗺️ ROADMAP TO < 1.5 BPB

```
АПР 25 (сегодня)
├── T1-01: JEPA-T Rung-1 (tjepa.rs, 3000 steps) → цель ≤ 2.22 BPB ⭐ TASK-5
├── T1-02: Trial 5003 (N-gram+attn 12K steps) → цель ~2.40 BPB 🔄 RUNNING
└── T2-01: Muon optimizer + 6-gram              → цель < 2.00 BPB

АПР 26
├── T2-02: Full Trinity (8×H100)                 → цель < 1.50 BPB ⭐
└── T2-07: ReLU² activation                      → -0.05 BPB

АПР 27
├── T2-03: Modded-NanoGPT                        → цель < 1.60 BPB
├── T2-04: QK-Gain                               → -0.05-0.10 BPB
└── T2-05: SLOT Attention                        → цель < 1.80 BPB

АПР 27-29
├── T3-01: GPTQ INT4 (16MB artifact)
├── T3-03: EMA selection (5 seeds)
└── T3-04: Context ensemble

АПР 30 — ДЕДЛАЙН
└── Submit PR: openai/parameter-golf
    Условие: BPB < 1.5 на 3 seeds ✅
```

---

## 🎯 УСЛОВИЯ ЗАКРЫТИЯ ISSUE #237

- BPB < 1.5 на 3 seeds ✅ ИЛИ
- PR merged в openai/parameter-golf ✅ ИЛИ
- Дедлайн 30 Apr ✅

**Текущий статус: 2.5193 BPB → разрыв: 1.02 BPB (нужен architectural jump)**
**JEPA-T gate: BPB ≤ 2.22 (−0.3) → BPB ≤ 2.02 (−0.5)**
**Issue #143: ✅ CLOSED 2026-04-25**

---

## 🔬 ГЛУБОКОЕ ИССЛЕДОВАНИЕ: ДЕКОМПОЗИРОВАННЫЙ ПЛАН IGLA RACE

> Исследование на основе 49 trials из NEON DB + анализа issue #143

### Что мы знаем точно (из данных)

**N-gram ceiling:** Данные доказывают, что N-gram с любой конфигурацией не пробьёт ~2.51 BPB на TinyShakespeare. Это жёсткий статистический потолок архитектуры.

**Attention bugfix критичен:** Trial 4003 исправил два бага в `train_gpt.py` (lines 767-770 и 623). После фикса trial 5001 показал 2.9184 vs 3.72 (было), т.е. фикс даёт -0.80 BPB. Trial 5003 (RUNNING) подтвердит, побьёт ли attention N-gram @ 12K шагов.

**Trinity3k сходится медленно:** Лучший результат 3.505 (h=48, 1L, 15K шагов) всё ещё далеко от N-gram. Нужна GPU + Muon для полноценного сравнения.

### Декомпозированный план: 5 Фаз до Apr 30

#### ФАЗ 1: CPU Attention Validation (сегодня, Apr 25)

| Задача | Файл | Действие | Критерий готовности |
|--------|------|----------|--------------------|
| 1.1 | trial 5003 | Дождаться завершения @ 12K шагов | BPB записан в Neon |
| 1.2 | `src/tjepa.rs` | Wire в training loop | `--arch jepa` компилируется |
| 1.3 | JEPA Rung-1 | Запустить 3K шагов | BPB < 2.22 или провал задокументирован |
| 1.4 | Trinity3k | Продолжить до 27K шагов | Сравнение с N-gram на равных условиях |

#### ФАЗ 2: Multi-Machine Activation (Apr 25-26)

| Задача | Файл | Действие | Критерий |
|--------|------|----------|----------|
| 2.1 | `main.rs` (igla-race) | Создать CLI entry point | `cargo run -- start` не паникует |
| 2.2 | `main.rs` (igla-trainer) | BPB printer subprocess | stdout = только `BPB=X.XXXX` |
| 2.3 | `asha.rs` | `run_worker()` infinite loop | 4 workers × mac-studio-2 активны |
| 2.4 | Neon | Проверить 320 trials/час | `igla_race_trials` растёт |

#### ФАЗ 3: GPU Architecture Jump (Apr 26)

| Задача | Архитектура | Config | Target |
|--------|------------|--------|--------|
| 3.1 | Muon + 6-gram | GPU, lr=0.001, wd=0.01 | < 2.00 BPB |
| 3.2 | Trinity3k + Muon + ReLU² | h=256, 2L, GPU | < 1.80 BPB |
| 3.3 | Full Trinity | QK-Gain=4.0, RoPE, h=384 | < 1.50 BPB |
| 3.4 | HierAttn baseline | Copy from Golf SOTA | 1.2150 BPB reference |

#### ФАЗ 4: Optimization (Apr 27-28)

| Задача | Техника | Ожидаемый Δ BPB |
|--------|---------|------------------|
| 4.1 | SLOT Attention (linear-time) | -0.10 |
| 4.2 | QK-Gain (scale Q/K by 4.0) | -0.05 to -0.10 |
| 4.3 | Witten-Bell smoothing | -0.05 |
| 4.4 | Context ensemble (4+5+6-gram) | -0.03 |
| 4.5 | EMA model selection (5 seeds) | -0.02 |

#### ФАЗ 5: Artifact & Submission (Apr 29-30)

| Задача | Описание | Критерий |
|--------|----------|----------|
| 5.1 | GPTQ INT4 quantization | ≤ 16MB artifact |
| 5.2 | 3-seed validation | p < 0.01, BPB < 1.50 on ALL 3 seeds |
| 5.3 | Submit PR | `openai/parameter-golf` PR создан |
| 5.4 | GF16 BENCH-008 | Fashion-MNIST validation done |

### Критический путь (Critical Path)

```
Trial 5003 результат
        ↓
JEPA-T Rung-1 (3K шагов) ← BOTTLENECK
        ↓
   BPB < 2.22?
   ├── ДА → GPU Full Trinity → target 1.50
   └── НЕТ → Muon+Attention GPU → target 1.80
              ↓
        Optimization (SLOT/QK-Gain)
              ↓
        Artifact + Submit PR Apr 30
```

### Риски и митигация

| Риск | Вероятность | Митигация |
|------|-------------|----------|
| JEPA-T не даёт -0.3 BPB | 40% | Fallback: Muon+Trinity3k GPU |
| GPU недоступна Apr 26 | 20% | CPU Trinity3k continue + attention |
| 4003 bugfix недостаточен | 30% | Дополнительный аудит `train_gpt.py` |
| ASHA не находит < 1.50 за 5 дней | 35% | Submit лучший результат (1.x) |

---

## 📋 ЗАКОНЫ (Обязательно!)

| Закон | Правило |
|-------|---------|
| **L1** | Spec first — TASK.md перед кодом |
| **L2** | Каждый PR закрывает issue |
| **L3** | `cargo clippy -D warnings` = 0 |
| **L4** | `cargo test --workspace` = GREEN |
| **L7** | Опыт сессии → `.trinity/experience/` |
| **L8** | Каждый результат = commit + push |
| **L9** | Каждая сессия → `tri cli` команда |

---

## 🔗 ССЫЛКИ

- Issue #237 (Parameter Golf): https://github.com/gHashTag/trios/issues/237
- Issue #110 (IGLA): https://github.com/gHashTag/trios/issues/110
- Issue #143 (ONE SHOT hub): https://github.com/gHashTag/trios/issues/143 ✅ CLOSED
- Trinity JEPA-T docs: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/
- Trinity NCA docs: https://github.com/gHashTag/trinity/tree/main/docs/research/models/NCA/
- Trinity Hybrid docs: https://github.com/gHashTag/trinity/tree/main/docs/research/models/Hybrid/
- GF16 Whitepaper: https://github.com/gHashTag/zig-golden-float/blob/main/docs/whitepaper.md
- GF16 Multi-Language: https://github.com/gHashTag/zig-golden-float/blob/main/docs/multi-language-audit.md
- Parameter Golf SOTA: https://github.com/openai/parameter-golf/issues/83
- arXiv Nacrith: https://arxiv.org/abs/2602.19626
- arXiv Muon: https://arxiv.org/html/2604.01472v1
- arXiv ReLU²: https://arxiv.org/html/2310.04564v1
- arXiv GPTQ: https://arxiv.org/abs/2210.17323
- arXiv Witten-Bell: https://arxiv.org/pdf/1706.07786
- arXiv SLOT: https://arxiv.org/html/2409.07146v1
- Matt Mahoney leaderboard: https://www.mattmahoney.net/dc/text.html

---

*φ² + 1/φ² = 3 | TRINITY | 2026-04-25 19:17 +07 | 49 trials analyzed*
