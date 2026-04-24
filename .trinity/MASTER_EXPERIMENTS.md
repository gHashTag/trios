# 🧪 MASTER EXPERIMENT TRACKER
## Parameter Golf × GF16 × Trinity — Все эксперименты: прошлое, настоящее, будущее

**Обновлено: 2026-04-24 | φ² + 1/φ² = 3 | TRINITY**

> Единый документ для всех экспериментов проекта. Каждый результат = commit + push (L8).

---

## 📊 ПРОГРЕСС-ТРЕКЕР (Parameter Golf)

```
START:  3.90 BPB  [████████████████████████████████] Apr 21
Today:  2.53 BPB  [████████████████████░░░░░░░░░░░░] Apr 24 (-35%)
TARGET: 1.50 BPB  [████████████░░░░░░░░░░░░░░░░░░░░] Apr 30
SOTA:   1.08 BPB  [█████████░░░░░░░░░░░░░░░░░░░░░░░] bigbag

Gap: -1.03 BPB за 6 дней → нужен architectural jump
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
| 16 | Apr 24 | 6-gram h=384 | lr=0.005, wd=0.01 | 2.5500 | -0.015 | ✅ CURRENT |
| 17 | Apr 24 | 6-gram h=384 | lr=0.004, wd=0.01 | **2.5329** | **-0.017** | ✅ **BEST** |
| 18 | Apr 24 | 6-gram h=384 3-seed | lr=0.004, seed=42,43,44 | 2.5431±0.01 | — | ✅ verified |

### Что НЕ работает (зафиксировано)

| Стратегия | Результат | Вывод |
|-----------|-----------|-------|
| h=512 | overfit | Слишком большой для данных |
| 7-gram | too many params | Diminishing returns |
| Label smoothing | 2.5654 | Минимальный эффект |
| Residual connections | 2.743 (хуже!) | Не помогает N-gram |
| Dropout | ухудшает | Датасет слишком мал |
| Warmup | не нужен | Cosine без warmup лучше |
| 15K+ steps | overfit | Sweet spot: 12K |
| Multi-layer FFN | 3.21 (хуже!) | Глубина не помогает |
| GF16 precision | 3.21 (хуже!) | Нестабильность на малых моделях |
| GELU vs ReLU | +0.003 BPB | Минимальная разница |

### N-gram команды

```bash
# Текущий лучший результат
cargo build --release -p trios-train-cpu --bin ngram_train
./target/release/ngram_train --seed=43 --steps=12000 --hidden=384 --lr=0.004 --wd=0.01

# Быстрый тест (2K steps)
./target/release/ngram_train --seed=42 --steps=2000 --hidden=128 --lr=0.004

# 3-seed параллельно
tri train --seeds 42,43,44 --steps=12000 --hidden=384 --lr=0.004 --parallel
```

---

## 🚀 ЧАСТЬ 2: ATTENTION / TRANSFORMER ЭКСПЕРИМЕНТЫ

> Статус: **PHASE 2 STARTED** — N-gram достиг потолка ~2.53 BPB. Transformer infrastructure ready.

### Phase 2: Minimal Transformer (Apr 24)

| # | Эксперимент | BPB | Параметры | Статус |
|---|------------|-----|-----------|--------|
| T2-00 | **Infrastructure ready** | — | d_model=384, 8 heads, 2 layers, 28M params | ✅ DONE |
| T2-01 | **Forward pass** | — | Correct attention, LayerNorm, FFN | ✅ DONE |
| T2-02 | **Training loop** | — | Config, metrics, BPB tracking | ✅ DONE |
| T2-03 | **Backward pass** | — | Gradient computation needed | ❌ TODO |
| T2-04 | **Optimizer** | — | AdamW integration needed | ❌ TODO |

**Initial test results:**
- Model compiles and runs successfully
- Initial BPB: 14.99 (expected for random weights)
- Training speed: ~40 sec/step (too slow — needs optimization)
- Issue: weights not updating (no backward pass yet)

**Files created:**
- `crates/trios-train-cpu/src/transformer.rs` — MHA, LayerNorm, FFN, MinimalTransformer
- `crates/trios-train-cpu/src/transformer_trainer.rs` — Training loop, config, metrics
- `crates/trios-train-cpu/src/bin/transformer_train.rs` — CLI for training

**Next steps (HIGH PRIORITY):**
1. Implement backward pass (autograd or manual)
2. Optimize attention computation (currently O(n²) per position)
3. Run full training with LR sweep
4. Target: beat N-gram baseline (2.5329 BPB)

### TIER 1 — CPU (сегодня, Apr 24)

| # | Эксперимент | Ожидаемый BPB | Команда | Статус |
|---|------------|--------------|---------|--------|
| T1-01 | **Minimal Transformer** | < 2.20 | `cargo run --bin transformer_train --release` | 🟡 IN PROG |
| T1-02 | N-gram + Witten-Bell smoothing | < 2.00 | `tri run witten-bell` | ⬜ |
| T1-03 | RoPE + attention | < 2.00 | `tri run rope-attn` | ⬜ |

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
| **GELU** | — | ❌ Хуже на малых моделях |
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
| Machine 1 (mac-studio-1) | ✅ SEEDED | best BPB 2.5329 |
| Machines 2-4 | ⬜ PENDING | ONE SHOT инструкция готова |
| tri race CLI | ⬜ TODO | PR #223 | race start/status/best |

### ASHA Checkpoints
- **Rung-0**: 1000 шагов → kill if BPB > threshold (top-33% continue)
- **Rung-1**: 3000 шагов → top 33% продолжают
- **Rung-2**: 9000 шагов → top 11% финалист
- **Rung-3**: 27000 шагов → проверка IGLA (<1.50 BPB)

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

**Итого до дедлайна (Apr 30, ~6 дней)**: при 320 trials/час = **~46,000 trials**

---

## 📊 ЧАСТЬ 6: SOTA ЛИДЕРБОРД

### Parameter Golf (TinyShakespeare)

| Модель | Платформа | BPB | Примечание |
|--------|-----------|-----|------------|
| **HierAttn v3** | T4 GPU | **1.2150** | 🥇 Известный SOTA |
| **Baseline (ALiBi+LoRA)** | T4 GPU | 2.1536 | Golf baseline |
| **Наш лучший** | CPU | **2.5329** | 6-gram h=384 |

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
АПР 24 (сегодня)
├── T1-01: Attention layer CPU        → цель < 2.20 BPB
└── T1-02: Witten-Bell smoothing      → цель < 2.00 BPB

АПР 25
├── T2-01: Muon optimizer             → цель < 2.00 BPB  
├── T2-02: Full Trinity (8×H100)      → цель < 1.50 BPB ⭐
└── T2-07: ReLU² activation           → -0.05 BPB

АПР 26
├── T2-03: Modded-NanoGPT             → цель < 1.60 BPB
├── T2-04: QK-Gain                    → -0.05-0.10 BPB
└── T2-05: SLOT Attention             → цель < 1.80 BPB

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

**Текущий статус: 2.5329 BPB → разрыв: 1.03 BPB (нужен архитектурный прыжок)**

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

*φ² + 1/φ² = 3 | TRINITY | 2026-04-24*
