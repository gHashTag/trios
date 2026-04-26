# Gate-final Pre-Registration DRAFT (BPB < 1.50, 3 seeds)

> **THIS IS A DRAFT, NOT YET IMMUTABLE.**
> Filed: 2026-04-26 10:36 +07
> Author: `perplexity-computer-grandmaster` (preparation lane)
> Status: advisory — will be frozen as immutable only after Gate-2 first row lands on seed=43.

## Decomposition -0.35 BPB (1.85 -> 1.50)

7 levers, each independently falsifiable by ablation:

| # | Lever | Delta BPB | INV anchor |
|---|-------|-----------|------------|
| 1 | Second causal-attention layer (d_model=64, 4 heads, RoPE, qk_gain=phi^2) | -0.10..-0.18 | INV-13 refined |
| 2 | phi-scaled hidden round(phi*512)=828 in n-gram block | -0.05..-0.10 | INV-1 + ch24 Golden Width |
| 3 | EMA-stabilised val_BPB, beta=phi^-1 | -0.02..-0.04 | INV-6 |
| 4 | GF16 weight floor in last 30% steps | -0.03..-0.07 | INV-3 + INV-5 |
| 5 | Cosine schedule 54K -> 81K (~phi^3*30K) | -0.05..-0.10 | INV-1 lr-band |
| 6 | 3-seed ASHA promotion (configs survive on >=2/3 seeds) | -0.02..-0.05 | INV-2 (Proven) |
| 7 | Rainbow Bridge cross-seed sync | -0.01..-0.03 | INV-8 |
| **Sum** | | **-0.28..-0.57** | |

Load-bearing: levers 1 + 2 + 5 + 6. Belt-and-braces: 3, 4, 7.

## Hypothesis (G1 Popper)

Hybrid config (depth=2 attention + phi-scaled hidden + EMA + GF16 floor + 81K cosine + 3-seed in {42, 43, 44})
gives BPB < 1.50 on all 3 seeds @ step >= 4000, Welch t-test vs mu_0=1.55 gives p < 0.01 one-tailed.

## Six Falsifiers (any -> hypothesis publicly burned)

1. Any seed BPB >= 1.50 @ step >= 4000
2. Welch p >= 0.01
3. Fewer than 3 distinct seeds in ledger
4. lr/qk_gain outside phi-band
5. ASHA-promoted <-> final-eval drift > 0.05
6. INV-7 igla_found_criterion rejects set

## Lane Decomposition (R6)

| Lane | File | Owner | Hours |
|------|------|-------|-------|
| L-f1 | hybrid_attn.rs (second layer behind cfg.num_attn_layers) | igla-l-f1-twin-attn | 4 |
| L-f2 | hybrid_train.rs (phi-hidden + 3-seed loop + EMA + GF16 floor) | igla-l-f2-trainer | 6 |
| L-f3 | seed_emit.rs (3 rows on seeds {42, 43, 44}) | igla-l-f3-ledger | 1 |
| L-f4 | victory.rs invoke check_victory() on 3-row tail | igla-l-f4-victory | 1.5 |
| L-f5 | twin_attn_ema_floor.v Coq lemmas Admitted | igla-l-f5-coq | 4 |
| L-f6 | freeze procedure (this auditor lane) | phd-monograph-auditor | 0 |

## Freeze Procedure (section 11)

- Gate-2 <= 1.85 -> freeze section 2/4 verbatim as IMMUTABLE Gate-final Pre-Registration (new comment on #143)
- Gate-2 in (1.85, 2.00] -> Gate-final v2 with re-weighted levers section 6
- Gate-2 > 2.00 -> falsifier Gate-2 already fired, strategy reset
- DRAFT itself is never edited (R10).

## References

- Race issue: trios#143
- PhD ONE SHOT: trios#265
- Champion baseline: 2446855 (BPB=2.2393 @ 27K, seed=43)
- DOIs: 10.5281/zenodo.19227877, 10.5281/zenodo.18947017, 10.5281/zenodo.19227879

---
L-f6 DRAFT filed. Awaiting Gate-2 first row.
agent=perplexity-computer-grandmaster
2026-04-26T03:36Z
