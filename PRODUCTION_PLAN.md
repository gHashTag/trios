# TRIOS: Monetization & Production Roadmap

> Author: LEAD agent | Date: 2026-05-20 | Anchor: phi^2 + phi^-2 = 3

---

## Executive Summary

**Trios** is a deep-tech platform with 3 commercializable pillars:
1. **GoldenFloat SDK** — Coq-certified quantization formats for edge AI
2. **MCP Server Platform** — AI agent Git orchestration (29+ tools)
3. **IGLA RACE** — Formally verified ML training pipeline

Current state: research monorepo (63 crates, 34 PhD chapters, 297 Coq Qed).
Target: revenue-generating products within 6 months.

---

## Phase 1: Package & Ship (Month 1-2)

### 1.1 GoldenFloat SDK (`trios-golden-float`)

**Product**: Rust crate + C FFI + Python bindings for GF4/GF8/GF16 formats.

**Actions**:
- [ ] Extract `trios-golden-float` into standalone publishable crate
- [ ] Add C FFI header (`gf16.h`) for embedded/FPGA integration
- [ ] Add Python bindings via PyO3 (`pip install goldenfloat`)
- [ ] Publish to crates.io as `golden-float` (MIT/Apache-2.0 dual)
- [ ] Write benchmarks vs BF16/MXFP4/FP8 (criterion.rs)
- [ ] Create `examples/` directory: quantize-pytorch-model, fpga-inference
- [ ] Write API docs on docs.rs

**Revenue model**: Open-core (MIT for GF16, commercial license for GF32/GF64 + FPGA RTL)

### 1.2 MCP Server Platform (`trios-server`)

**Product**: Hosted MCP server for AI agent Git orchestration.

**Actions**:
- [ ] Extract MCP server into `trios-mcp-server` standalone binary
- [ ] Add authentication (API key + OAuth2)
- [ ] Add multi-tenant workspace isolation
- [ ] Deploy to Railway/Fly.io with managed PostgreSQL
- [ ] Create onboarding: `npx create-trios-mcp` or Docker one-liner
- [ ] Write OpenAPI spec for REST endpoints
- [ ] Create landing page with playground

**Revenue model**:
| Tier | Price | Limits |
|------|-------|--------|
| Free | $0 | 1 workspace, 100 req/hr |
| Pro | $29/mo | 10 workspaces, 10K req/hr |
| Enterprise | Custom | Unlimited, SSO, SLA |

### 1.3 Precision Router (`trios-precision-router`)

**Product**: Mixed-precision policy engine — decides which layers get GF16 vs ternary.

**Actions**:
- [ ] Add ONNX/PyTorch model importer
- [ ] Add sensitivity analysis per-layer (Hessian trace)
- [ ] Export quantized model in ONNX format
- [ ] Benchmark on Llama-3.2-1B, Phi-3-mini, Mistral-7B
- [ ] Publish as `precision-router` crate

**Revenue model**: Part of GoldenFloat SDK (commercial tier)

---

## Phase 2: Validate & Traction (Month 3-4)

### 2.1 DARPA Proposal

**Opportunity**: IGTC solicitation HR001124S0001 — 3000x energy efficiency.

**Actions**:
- [ ] Write 15-page proposal围绕 "GoldenFloat Ternary Inference Engine"
- [ ] Emphasize: Coq-certified bounds (no competitor has this)
- [ ] Include: 63 tok/s @ 1W measured result, path to 19,478x on UltraScale+
- [ ] Target: $500K-$2M Phase I contract

### 2.2 FPGA IP Core

**Product**: synthesizable Verilog IP core for GF16 ternary inference.

**Actions**:
- [ ] Extract RTL from trinity-fpga into standalone IP package
- [ ] Target Xilinx UltraScale+ and Intel Agilex
- [ ] Create evaluation kit (free for academic, $10K/yr commercial)
- [ ] List on Xilinx IP catalog
- [ ] Partner with Tiny Tapeout for MPW shuttle validation

**Revenue model**: IP license $50K-$200K per design-in

### 2.3 Developer Adoption

**Actions**:
- [ ] Write 5 blog posts: "Why GF16 beats MXFP4", "Coq proofs for ML", etc.
- [ ] Submit paper to NeurIPS/ICML workshop on efficient ML
- [ ] Create GitHub Discussions for community
- [ ] Publish HuggingFace integration: `from goldenfloat import quantize_gf16`
- [ ] Benchmark leaderboard (continuous)

---

## Phase 3: Scale & Revenue (Month 5-6)

### 3.1 SaaS Platform

**Product**: Managed IGLA RACE training platform.

**Actions**:
- [ ] Multi-tenant IGLA RACE with per-team isolation
- [ ] Web dashboard (extend existing HTMX dashboard)
- [ ] Billing integration (Stripe)
- [ ] Formally verified training reports (PDF with Coq seals)
- [ ] EU AI Act compliance reports (generated from Coq proofs)

**Revenue model**:
| Tier | Price | Features |
|------|-------|----------|
| Academic | Free | 1 GPU, public results |
| Research | $499/mo | 4 GPUs, private results |
| Enterprise | $2K+/mo | Dedicated fleet, custom formats |

### 3.2 Consulting & Training

**Actions**:
- [ ] Offer "Verified ML Quantization" workshop ($5K/day)
- [ ] Enterprise consulting: custom quantization for specific models
- [ ] Audit service: formal verification of existing quantization schemes
- [ ] Annual "Trinity SAI" conference/workshop

### 3.3 IP Licensing

**Actions**:
- [ ] File provisional patent on phi-structured exponent partition
- [ ] File provisional patent on Lucas-indexed mantissa widths
- [ ] Create IP licensing FAQ
- [ ] Target: Qualcomm, ARM, NVIDIA for GF16 IP evaluation

---

## Technical Prerequisites

### Before Phase 1:
1. **Separate crate from monorepo**: `trios-golden-float` → standalone repo
2. **CI/CD**: GitHub Actions for crates.io publish, PyPI publish, Docker build
3. **Benchmark suite**: criterion.rs vs BF16, FP8, MXFP4, NF4
4. **Documentation**: mdbook or docs.rs API docs
5. **License audit**: ensure all dependencies are MIT/Apache-2.0 compatible

### Before Phase 2:
1. **FPGA demo**: Working inference on QMTech board with GF16 model
2. **Video demo**: YouTube walkthrough of MCP server + AI agent workflow
3. **Benchmark paper**: Preprint on arXiv with reproducible results
4. **Reference customers**: 3 beta testers for MCP server

### Before Phase 3:
1. **Payment integration**: Stripe + usage metering
2. **SOC 2 Type I**: Security audit for SaaS
3. **Enterprise features**: SSO, audit logs, SLA
4. **Legal**: IP assignment, EULA, privacy policy

---

## Revenue Projections (Conservative)

| Stream | Month 6 | Month 12 | Month 24 |
|--------|---------|----------|----------|
| MCP SaaS | $2K/mo | $15K/mo | $80K/mo |
| GF SDK licenses | $0 | $10K | $100K |
| FPGA IP | $0 | $50K | $300K |
| DARPA/contracts | $0 | $100K | $500K |
| Consulting | $5K | $20K | $60K/yr |
| **Total** | **$7K** | **$195K** | **$1.8M/yr** |

---

## Key Metrics to Track

| Metric | Target (Month 6) |
|--------|-----------------|
| GitHub stars (golden-float) | 500+ |
| crates.io downloads | 1K+/mo |
| PyPI downloads | 500+/mo |
| MCP server paid users | 50+ |
| Benchmark citations | 5+ |
| DARPA proposal status | Submitted |
| FPGA eval kit signups | 20+ |

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| GF16 accuracy doesn't hold at scale | Medium | High | Benchmark on 7B+ models early |
| Competitor ships similar MCP product | Medium | Medium | First-mover with 29 tools; keep extending |
| DARPA proposal rejected | High | Low | Multiple submissions; EU Horizon alternative |
| FPGA vendor lock-in | Low | Medium | Target Xilinx + Intel + Lattice |
| Patent prior art | Medium | Medium | File provisionals immediately |

---

## Immediate Next Steps (This Week)

1. Create `goldenfloat` standalone crate (extract from `trios-golden-float`)
2. Set up `docs.rs` documentation
3. Write benchmarks vs BF16/FP8/MXFP4
4. Create landing page for MCP server
5. File provisional patent on phi-exponent partition
6. Write DARPA proposal abstract
7. Create PyPI package skeleton

---

*"The only number format with machine-checked precision guarantees."*
