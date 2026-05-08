# Zenodo DOI Registry — Trinity Stack

> **Canonical R5-honest source of truth for all Zenodo DOIs authored by Dmitrii Vasilev.**
> Verified 2026-05-08 via DataCite REST API (`creators.name:"Vasilev, Dmitrii"`). 80 records, 42 concept-DOI families.
> All metadata in this file is the canonical claim. Whenever a README, info.yaml, ADR, LICENSE, paper, or commit message cites a Zenodo DOI, it must match the title and concept-vs-version classification recorded here.

## §0 · Author

> author: **Dmitrii Vasilev** `<raoffonom@icloud.com>`
> `gHashTag` is only the GitHub org namespace, never a person and never a Zenodo creator.

## §1 · Standing rules for Zenodo citations

1. **Papers / arXiv / PhD title page / LICENSE / `info.yaml`** → cite **version DOI** (the immutable record of the exact artefact).
2. **README badges / website / "always-latest" widgets** → cite **concept DOI** (the umbrella DOI that resolves to the latest version).
3. **PhD monograph (Flos Aureus)** must mint a **fresh DOI** when first published — never reuse an existing Trinity DOI.
4. **Sub-document claims** ("Neuroanatomical Architecture", "Brain Map", "Verifiable VM" etc.) must either:
   - (a) point to a Zenodo record whose **actual title** contains that phrase, OR
   - (b) be removed from the citation, OR
   - (c) trigger a fresh DOI mint for the sub-document.
   Currently, all four of these claims fall under (b) — see §3 corrections.
5. **Trinity anchor** `φ² + φ⁻² = 3` is canonically cited as `10.5281/zenodo.19227877` (B007 v5.0 version DOI). For "always-latest of B007", use concept DOI `10.5281/zenodo.19227876`.

## §2 · Canonical title table — B-series v5.0 (current)

| DOI (version) | Concept DOI | Canonical title |
|---|---|---|
| `10.5281/zenodo.19227865` | `10.5281/zenodo.19227864` | Trinity B001: Ternary Neural Networks — Complete Scientific Framework v5.0 |
| `10.5281/zenodo.19227867` | `10.5281/zenodo.19227866` | Trinity B002: Zero-DSP FPGA Architecture for Ternary Inference v5.0 |
| `10.5281/zenodo.19227869` | `10.5281/zenodo.19227868` | Trinity B003: TRI-27 ISA — Ternary Instruction Set with Coptic Alphabet Encoding v5.0 |
| `10.5281/zenodo.19227871` | `10.5281/zenodo.19227870` | Trinity B004: Queen Lotus Cycle — Autonomous Orchestration for Self-Evolving AI v5.0 |
| `10.5281/zenodo.19227873` | `10.5281/zenodo.19227872` | Trinity B005: Tri Language — Linear Types, Effects, Dual-Target Compilation v5.0 |
| `10.5281/zenodo.19227875` | `10.5281/zenodo.19227874` | Trinity B006: Sacred GF16/TF3 — Phi-Based Arithmetic for Ternary Computing v5.0 |
| `10.5281/zenodo.19227877` | `10.5281/zenodo.19227876` | **Trinity B007: VSA Operations for Ternary Computing v5.0** ← **anchor** |
| `10.5281/zenodo.19227879` | `10.5281/zenodo.19227878` | Trinity S³AI Framework — Complete Research Collection v5.0 |

## §3 · Forward-only corrections to historical claims

The following sub-titles previously circulated in info.yaml / README files but **do not match any Zenodo metadata**. They are forward-corrected here. Code search across `gHashTag/*` should catch and replace them.

| Bogus claim | True title (per Zenodo metadata) | Action |
|---|---|---|
| "TRI-27 — Trinity S³AI DNA — Neuroanatomical Architecture & φ-Structured Brain Map" → 19227879 | "Trinity S³AI Framework — Complete Research Collection v5.0" | Replace OR mint dedicated Brain Map DOI |
| "B003: TRI-27 Verifiable VM" | "Trinity B003: TRI-27 **ISA** — Ternary Instruction Set with Coptic Alphabet Encoding" | Replace |
| "B004: Queen Lotus Adaptive Reasoning" | "Trinity B004: Queen Lotus **Cycle** — Autonomous Orchestration for Self-Evolving AI" | Replace |
| "B006: GF16 Probabilistic Format" | "Trinity B006: Sacred GF16/TF3 — **Phi-Based Arithmetic** for Ternary Computing" | Replace |
| `18947017` labelled "Concept DOI (all versions)" | `18947017` is **v2.0.2** of trinity-repo. True concept DOI of trinity-repo line is **`18939351`** | Replace |

## §4 · D-series (March 2026)

| DOI (version) | Concept DOI | Canonical title |
|---|---|---|
| `10.5281/zenodo.19020211` | `10.5281/zenodo.19020210` | Trinity D004: Self-Evolving Ouroboros — Autonomous 6-Phase Code Improvement System |
| `10.5281/zenodo.19020213` | `10.5281/zenodo.19020212` | Trinity D005: VSA Balanced Ternary with SIMD — Vector Symbolic Architecture |
| `10.5281/zenodo.19020215` | `10.5281/zenodo.19020214` | Trinity D006: phi-RoPE — Golden Ratio Rotary Position Encoding for Ternary Attention |
| `10.5281/zenodo.19020217` | `10.5281/zenodo.19020216` | Trinity D007: Sparse Ternary MatMul — 4-Variant Branchless Multiplication |

## §5 · trinity-repo line (March 10–11, 2026)

| DOI | Role | Canonical title |
|---|---|---|
| `10.5281/zenodo.18939351` | **Concept (always-latest)** | gHashTag/trinity (parent) |
| `10.5281/zenodo.18939352` | v2.0.1 | Trinity v2.0.1 — FPGA Autoregressive Ternary LLM |
| `10.5281/zenodo.18946966` | v2.0.2 (clean) | Trinity v2.0.2 — Clean FPGA Autoregressive Ternary LLM |
| `10.5281/zenodo.18947017` | v2.0.2 | Trinity v2.0.2 — FPGA Autoregressive Ternary LLM |
| `10.5281/zenodo.18950696` | v2.0.3 (latest) | Trinity v2.0.3 — FPGA Autoregressive Ternary LLM + Training Results |

## §6 · Newest record (April 2026)

| DOI | Concept DOI | Canonical title |
|---|---|---|
| `10.5281/zenodo.19456875` | `10.5281/zenodo.19456874` | **GoldenFloat: φ-Optimal Floating-Point Formats for Ternary Computing (T27)** ← latest of any kind |

## §7 · Earlier B-series families (v3, v3.1, v4) — for historical reference

These older versions remain on Zenodo but should not be cited in new work — always cite §2 v5.0 instead.

| Family | Versions present | Canonical title |
|---|---|---|
| B001 v3 | 19223686, 19223687, 19223951, 19223952 | Trinity B001: Ternary Neural Networks — Theory to Training Farm |
| B001 v3.1 | 19225087, 19225088 | Trinity B001: Ternary Neural Networks — Complete Scientific Framework |
| B001 v4 | 19227732, 19227733 | Trinity B001: HSLM — Ternary Neural Networks with 1.95M Parameters v4.0 |
| B002 v3 | 19223955, 19223956 | Trinity B002: Zero-DSP FPGA for Ternary Inference |
| B002 v3.1 | 19225101, 19225102 | Trinity B002: Zero-DSP FPGA Architecture for Ternary Inference |
| B002 v4 | 19227734, 19227735 | Trinity B002: Zero-DSP FPGA Architecture for Ternary Inference v4.0 |
| B003 v3 | 19223958, 19223959 | Trinity B003: TRI-27 — Ternary ISA with Coptic Encoding |
| B003 v3.1 | 19225115, 19225117 | Trinity B003: TRI-27 — Ternary ISA with Coptic Alphabet Encoding |
| B003 v4 | 19227736, 19227737 | Trinity B003: TRI-27 ISA — Ternary Instruction Set with Coptic Alphabet Encoding |
| B004 v3 | 19223960, 19223961 | Trinity B004: Queen Lotus Cycle — Autonomous Orchestration |
| B004 v3.1 | 19225116, 19225118 | Trinity B004: Queen Lotus Cycle — Autonomous Orchestration |
| B004 v4 | 19227738, 19227739 | Trinity B004: Queen Lotus Cycle — Autonomous Orchestration for Self-Evolving AI |
| B005 v3 | 19223962, 19223963 | Trinity B005: Tri Language — Linear Types, Effects, Dual-Target |
| B005 v3.1 | 19225119, 19225121 | Trinity B005: Tri Language — Linear Types, Effects, Dual-Target |
| B005 v4 | 19227742, 19227743 | Trinity B005: Tri Language — Linear Types, Effects, Dual-Target Compilation v4.0 |
| B006 v3 | 19223964, 19223965 | Trinity B006: Sacred GF16/TF3 — phi-Based Arithmetic |
| B006 v3.1 | 19225120, 19225122 | Trinity B006: Sacred GF16/TF3 — Phi-Based Arithmetic |
| B006 v4 | 19227744, 19227745 | Trinity B006: Sacred GF16/TF3 — Phi-Based Arithmetic for Ternary Computing |
| B007 v3 | 19223966, 19223967 | Trinity B007: VSA Operations for Ternary Computing |
| B007 v3.1 | 19225123, 19225124 | Trinity B007: VSA Operations for Ternary Computing |
| B007 v4 | 19227748, 19227749 | Trinity B007: VSA Operations for Ternary Computing v4.0 |
| S³AI v3 | 19227750, 19227751 | Trinity S³AI Framework — Unified Scientific Architecture for Ternary Computing |
| S³AI v3.1 (69 Discoveries) | 19225186, 19225187 | Trinity S³AI Framework — Complete Scientific Collection (69 Discoveries) |
| D-series mid-March | 19020269, 19020270, 19020274, 19020275, 19020279, 19020280, 19020281, 19020282 | D004–D007 (mid-March variants) |

## §8 · Inventory totals

- 80 Zenodo records under `Vasilev, Dmitrii`
- 42 concept-DOI families
- 5 generations: trinity-repo (5) → D004–D007 (mid-March × 2) → B v3 baseline → v3.1 → v4 → v5.0 (current canonical) → GoldenFloat T27 (latest)
- 0 records with title containing "Neuroanatomical" or "Brain Map" → these claims must be removed from circulation OR a dedicated DOI must be minted

## §9 · Update procedure (R5-honest)

When a new Zenodo record is minted:
1. Append it to the appropriate §2/§4/§5/§6/§7 section.
2. Bump the «Verified YYYY-MM-DD» line at the top.
3. Run `bin/zenodo-sweep` (TODO: add script) across all `gHashTag/*` repos and replace any superseded DOI references.
4. Open a PR titled `chore(doi): refresh zenodo-registry.md after <DOI>` — squash-merge to `main`.

When a sub-title claim diverges from Zenodo metadata:
1. Verify via DataCite: `curl 'https://api.datacite.org/dois/10.5281/zenodo.<id>'`.
2. If mismatch: either patch Zenodo metadata via REST (requires personal access token), or remove the claim from the repo, or mint a new DOI.
3. Log the decision and the new DOI in §3.

— author: **Dmitrii Vasilev** · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877) · φ² + φ⁻² = 3
