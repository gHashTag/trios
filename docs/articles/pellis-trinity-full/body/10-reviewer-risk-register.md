## Reviewer-risk register

| Risk | Severity | Mitigation in article |
|---|---|---|
| Merge-conflict markers in `docs/ARCHITECTURE.md` | High | Do not cite as a finished artifact; cite audited facts only |
| Catalog42 overclaim | High | Lock wording to 19 closed / 23 UnderRevision; see §"Proof-status lock" |
| L01/L02/L03/Q03/Q05 marked verified | High | Downgraded to UnderRevision; see §"Catalog42 row-by-row status" |
| Bonferroni p-value > 1 | High | Capped at $\min(1, n \cdot p) = 1$; see §"Statistical multiplicity" |
| Prop 8.2 convention conflict | High | Two-convention split (residual vs. exponent); see §"Proposition 8.2" |
| Bare-bracket citation placeholders carried over from earlier drafts (the "link placeholders" pattern) | High | Real URLs in §"References"; QA grep blocks the bare-bracket placeholder pattern |
| Wilson & Kogut wrong page | Medium | Use *Physics Reports* **12**, 75–199 (1974) |
| Truncated v21 appendix sentences | Medium | Restored Sacred ALU and integration-label paragraphs in Strand III |
| TRI-27 banks/opcodes overclaim | High | Removed "3 banks" and "36 opcodes" unless a separate canonical source is added |
| Brain-module count inconsistency | Medium | Present two taxonomies, mark implementation-state |
| Sacred ALU performance overclaim | Medium | Keep LUT/FF/DSP; mark Fmax/latency/throughput as estimates |
| v21 label absent from repos | Medium | Treat v21 as integration label, not repo-native version string |
| Ch.36 TRI-1 skeleton | Medium | State skeleton status, not wired into main build |
| Pseudo-Latin / microtext / rasterized labels in figures | High | Regenerate in-source as vector PDF text; see `figures/manifest.json` |
| Orange highlight annotations in PDF | High | Renderer must emit `/Annots` count 0 unless hyperlinks present; see QA config |
