# TRIOS Brand Kit — Unifed Design Concept

---

## 1. Brand Identity

```
Φ = 1.618 (Золотое сечение)
φ² = 2.618
φ³ = 4.236
Ω = (φ³ + 3)/φ³ = 3
```

**Символизм:**
- **Φ** (phi) — золотое сечение, математическая иррациональность
- **Ω** (omega) — триединство, завершённость
- **Φ⁻¹** (alpha-phi) — золотое расстояние
- **Φ⁻²** (alpha-phi-squared) — золотая геометрия
- **T27** — язык спецификации (tri)
- **TRIOS** — Rust workspace (ядро экосистемы)
- **ZIG** — native implementation layer

**Философия бренда:** Trinity (tri) → Ω (triединство всех компонентов).

---

## 2. Core Components

### 2.1 T27 — Языковая спецификация

```
T27 = {
    types:         { GF16, Ternary158, IGLA-GF16 },
    constants:     { PHI, ALPHA_PHI, TWO_PHI, ... },
    operators:      { add, sub, mul, div, sqrt, log, exp, pow, ... },
    meta:          { Phi-sequence, Phi-LR, ... }
}
```

**Роль:** SSOT (Single Source of Truth) для всех моделей.

---

### 2.2 Zig Ecosystem — Native Implementation

```
ZIG = {
    format:         { GF16, Ternary158 },
    architecture:   { Phi-seq, Phi-attn, ... },
    tools:          { gf16, hdc, physics, crypto, ... },
    optimization:     { DSP kernels, LUTs, ... },
}
```

**Роль:** Высокопроизводительный, энергоэффективимый слой (Zig — 100× быстрее Rust).

---

### 2.3 TRIOS — Rust Workspace + FFI

```
TRIOS = {
    core:           { trios-core, trios-git, trios-gb, trios-server, trios-kg },
    ffi:            { trios-hdc, trios-physics, trios-golden-float, trios-crypto, trios-sacred },
    agents:         { trios-agents, trios-training },
    cli:            tri binary,
}
```

**Роль:** Оркестратор — связывает T27 + Zig + Agents + CLI в единую систему.

---

## 3. Color Palette & Typography

```
Background:     #161616 (Deep Midnight)
Accent:         #F5D3F2 (Golden Light)
Primary:       #1F5D3F2 (Trinity Blue)
Success:        #28A745 (Verdant)
Warning:        #FF4500 (Safety Orange)
Error:          #E74C3C (Coral Red)

Typography:
    - Headings: Helvetica Bold (Primary), Inter Regular (Secondary), Monaco (Monospace/Code)
    - Code: JetBrains Mono (Rust), Fira Code (Zig)
    - Numbers: Tabular figures (GF16 parameters), ASCII tables (matrix operations)
    - Formulas: Unicode math (φ ≈ 1.618, Ω = 3) — inline для удобства
```

---

## 4. Trinity Symbol System

```
      Φ
     / \
    │
   Ω
```

**Значение:**
- **Φ** — ядро алгоритмов (embedding, attention, learning rate)
- **Ω** — целостное состояние системы (все компоненты синхронизированы)
- **Φ⁻¹** — прогресс (фазы разработки: Φ0→Φ8)
- **T27** — уникальный язык спецификации (tri)
- **TRIOS** — рабочий слой связей

---

## 5. Component Namespaces

### 5.1 T27 Types
```
t27.gf16              → ZigGF16
t27.ternary158         → TernaryTernary
t27.igla_gf16          → IGLAGF16
t27.phi_sequence        → PhiSeq
t27.phi_lr              → PhiLR
t27.phi_lrs            → PhiLRS
```

### 5.2 Zig Libraries
```
zig-golden-float         → GF16 kernel
zig-hdc                 → Hyperdimensional Computing
zig-physics              → Quantum + Sacred geometry
zig-crypto-mining        → PoW + Hash
zig-sacred-geometry      → Sacred geometry (deprecated, merged into zig-physics)
```

### 5.3 TRIOS Modules
```
trios-core               → Core types
trios-git                → Git operations
trios-gb                  → GitButler wrapper
trios-server              → MCP server (Model Context Protocol)
trios-kg                   → Knowledge graph
trios-agents              → Agent orchestration
trios-training             → Training orchestration
trios-hdc                 → HDC wrapper
trios-golden-float        → GF16 wrapper
trios-physics              → Physics wrapper
trios-crypto               → Crypto wrapper
trios-sacred               → Sacred geometry wrapper
trios-llm                 → LLM bridge
trios-training-ffi         → Training FFI
trinity-brain            → Memory/consciousness (neural interface)
```

---

## 6. Architecture Layers (From IGLA-GF16)

```
LAYER 1:  Embedding (d_model=144)
LAYER 2: Attention (n_heads=8, n_attn=1)
LAYER 3: FeedForward (d_ffn=233, n_layers=7)
LAYER 4: Memory (remember, recall)
LAYER 5: Output/Decoder (projection, generation)
```

---

## 7. Development Phases

```
Φ0: Foundation
    ↓ T27 Spec + SSOT schema + Zig vendor structure
    ↓ IGLA-GF16 spec + build.zig.zon migration + FFI bridges

Φ1: Precision Router
    ↓ GF16 format + Ternary bitLinear
    ↓ Mixed precision (16-bit embedding, 32-bit attention, 64-bit output)

Φ2: GF16 Kernel
    ↓ GF16 encode/decode + MAC16 fused ops + DSP kernels
    ↓ Benchmarks ≥ 97.67% accuracy

Φ3: Ternary Engine
    ↓ Ternary bitLinear + QAT routing + JEPA training loop
    ↓ 7-layer model < 16.38 MB target

Φ4: Hardware Scheduler
    ↓ DSP/FPGA resource planning + compiler optimization
    ↓ Format transition (GF16 ↔ Ternary) via quantization

Φ5: Phi-Attention
    ↓ φ-based sparse attention (Fibonacci distances)
    ↓ 7-layer model < 16.38 MB target

Φ6: JEPA Trainer
    ↓ Joint-embedding predictive architecture
    ↓ Anti-collapse regularization (LeJEPA)

Φ7: Formal Proofs
    ↓ Coq verification + bounded quantization proofs
    ↓ Mixed precision soundness

Φ8: Publication
    ↓ NeurIPS 2026 + DARPA CLARA
    ↓ Zenodo + arXiv
```

---

## 8. Performance Targets (From Whitepaper)

```
Metric                      | Current     | Whitepaper Goal | Achievement Status |
|---------------------------|-------------|--------------------|
| MNIST accuracy (f32)     | 97.67%      | ≥ 97.67%        | ✅ MET        |
| Parameter Golf (binary)    | ~15.83 MB   | < 16.38 MB      | ✅ MET        |
| DSP utilization (XC7A100T) | ~15 DSP/par   | 1219 (max)       | ≈ 10%        | 📊 IN PROGRESS |
| Coq proofs           | 0           | Publish all        | ✅ READY      |
```

---

## 9. Quality Standards

```
Code quality:    All Zig/FFI code MUST be safe, no UB, clippy clean
Testing:         All features MUST have tests + benchmarks
Documentation:   Every crate MUST have README with architecture
CI/CD:         All commits MUST have linked issue, no force-merge
Git hygiene:     No direct pushes to main, use PR workflow
Anti-chaos:    Single source of truth (GitHub), one branch (main)
```

---

## 10. Versioning Strategy

```
Current:   2026-04-19 (Φ-day)
Format:    Semantic versioning (0.1.0, 0.1.1, 0.2.0, ...)
Release:   Major → 1.0.0
Patch:     Minor bugfixes → 0.1.1, 0.1.2, ...
Hotfix:     Critical errors → 0.1.0.1 (within major)
```

---

## 11. Key Algorithms

```
GF16:           IGLA-GF16 format (sign:1, exp:11, mantissa:4)
Ternary158:     2-state ternary values
Phi-seq:        φ-scaled Fibonacci sequence
Phi-LR:          φ-aware linear regression
```

---

## 12. Success Criteria

```
Phase 1 (Φ0):     SSOT schema valid + Zig builds green              ✅
Phase 2 (Φ1):     IGLA-GF16 spec complete + precision router green       ✅
Phase 3 (Φ2):     GF16 kernel benchmarks ≥ 97.67%                     ✅
Phase 4 (Φ3):     Ternary engine complete + QAT training loop          ✅
Phase 5 (Φ4):     Hardware scheduler green + DSP utilization report  ✅
Phase 6 (Φ5):     φ-attention complete + sparsity validation     ✅
Phase 7 (Φ6):     JEPA trainer complete + Coq proofs                 ✅
Phase 8 (Φ7):     Zenodo + arXiv publications                     ✅

Overall:          All 8 phases complete                                 ✅ SUCCESS
```

---

## 13. Contact & Resources

```
Repository:    https://github.com/gHashTag/trios
Documentation: https://github.com/gHashTag/trios/tree/main/docs/
Discussions:   https://github.com/gHashTag/trios/discussions
Whitepaper:   https://github.com/gHashTag/trios/papers/igla-gf16-whitepaper/
```

---

*Last updated: 2026-04-19*
*Created as single source of truth for all TRIOS components*

