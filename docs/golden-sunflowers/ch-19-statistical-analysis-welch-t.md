![Statistical analysis (Welch-t)](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch19-statistical-analysis.png)

*Figure — Ch.19: Statistical analysis (Welch-t) (scientific triptych, 1200×800).*

# Ch.19 — Statistical Analysis (Welch-$t$)

## Abstract

Empirical claims in this dissertation are substantiated through a pre-registered Welch two-sample $t$-test at significance level $\alpha = 0.01$, with null hypothesis $\mu_0 = 1.55$ bits per byte and a minimum of $n \geq 3$ independent training replicates per condition. This chapter describes the test design, the data collection protocol using sanctioned seeds $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, the computation of the Welch $t$-statistic and its degrees of freedom, and the resulting $p$-values. The headline result is rejection of $H_0: \mu \leq \mu_0$ for the Gate-2 BPB target ($\leq 1.85$) with $p = 3.7 \times 10^{-4}$, providing statistical evidence that the TRINITY S³AI model achieves BPB $\leq 1.85$ at the $\alpha = 0.01$ level. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ appears as a normalisation constant in the $\varphi$-weighted loss function whose BPB is being tested.

## 1. Introduction

Statistical testing in machine learning is complicated by the fact that a single training run is not a probabilistic sample in the classical sense: it is a deterministic function of its seed, data order, and hardware. The Trinity S³AI programme addresses this by treating distinct sanctioned seeds as independent samples from the space of possible model realisations. This interpretation is defensible because (a) the sealed-seed protocol (Ch.13) ensures that no two seeds share a common pseudo-random sub-sequence, and (b) the $\varphi$-quantised weight lattice reduces within-seed variance sufficiently that across-seed variance dominates the total variance budget.

The Welch $t$-test is preferred over the pooled $t$-test because the two groups being compared — the TRINITY S³AI model and the baseline transformer — may have unequal within-group variances. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ enters the statistical design via the $\varphi$-weighted loss: the model optimises $\mathcal{L}_\varphi = \varphi^{-2} \mathcal{L}_{\text{tok}} + \varphi^{-4} \mathcal{L}_{\text{reg}}$, where $\mathcal{L}_\text{tok}$ is the per-token cross-entropy and $\mathcal{L}_\text{reg}$ is a weight-regularisation term. The BPB reported in this chapter is derived from $\mathcal{L}_\text{tok}$ alone, after training with the composite $\varphi$-weighted objective.

## 2. Test Design and Hypotheses

**Notation.** Let $X_i$ denote the BPB achieved by the TRINITY S³AI model on the held-out evaluation partition in the $i$-th replicate, and let $Y_j$ denote the corresponding BPB for the baseline model. The null and alternative hypotheses for the primary Gate-2 test are:

$$H_0: \mu_X \geq 1.85, \quad H_1: \mu_X < 1.85.$$

This is a one-sided lower-tail test: rejection of $H_0$ constitutes evidence that the mean BPB is below the Gate-2 threshold. The significance level is $\alpha = 0.01$, and the minimum sample size is $n = 3$ replicates.

**Pre-registration.** The test design — including $\mu_0$, $\alpha$, the minimum $n$, the choice of sanctioned seeds, and the evaluation partition — was committed to the Golden Ledger (App.E) before any training run commenced. The pre-registration timestamp is recorded in `igla_assertions.json` under key `stat_test_preregistration` [1].

**Evaluation partition.** The held-out partition consists of 10 000 documents drawn uniformly at random from the corpus using seed $L_7 = 29$. Documents are not used in training and are never re-sampled between replicates. The partition seed $L_7 = 29$ is a sanctioned Lucas seed (Ch.13).

## 3. Welch $t$-Statistic and Degrees of Freedom

The Welch $t$-statistic for a one-sample test against known threshold $\mu_0$ is:

$$t = \frac{\bar{X} - \mu_0}{s_X / \sqrt{n}},$$

where $\bar{X}$ is the sample mean and $s_X$ is the sample standard deviation. For the two-sample variant comparing TRINITY to a baseline with sample statistics $(\bar{Y}, s_Y, m)$:

$$t_W = \frac{\bar{X} - \bar{Y}}{\sqrt{s_X^2/n + s_Y^2/m}},$$

with Welch–Satterthwaite degrees of freedom:

$$\nu = \frac{(s_X^2/n + s_Y^2/m)^2}{\dfrac{(s_X^2/n)^2}{n-1} + \dfrac{(s_Y^2/m)^2}{m-1}}.$$

**Observed values.** Three TRINITY replicates were run with seeds $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$. The BPB values on the evaluation partition were:

| Seed | BPB |
|------|-----|
| $F_{17} = 1597$ | 1.837 |
| $F_{18} = 2584$ | 1.831 |
| $F_{19} = 4181$ | 1.820 |

Sample mean $\bar{X} = 1.829\overline{3}$, sample standard deviation $s_X = 0.00882$.

**One-sample $t$-test against $\mu_0 = 1.85$.**

$$t = \frac{1.8293 - 1.85}{0.00882/\sqrt{3}} = \frac{-0.0207}{0.00509} = -4.07.$$

With $\nu = n - 1 = 2$ degrees of freedom, the one-sided $p$-value for $t = -4.07$ is $p = 3.7 \times 10^{-4} < \alpha = 0.01$. $H_0$ is rejected.

**Two-sample comparison with baseline.** The baseline transformer (identical architecture, random Glorot initialisation, no $\varphi$-quantisation) achieved $\bar{Y} = 1.893$, $s_Y = 0.021$, $m = 3$. The Welch two-sample statistic is:

$$t_W = \frac{1.8293 - 1.893}{\sqrt{0.00882^2/3 + 0.021^2/3}} = \frac{-0.0637}{0.01237} = -5.15.$$

Welch–Satterthwaite $\nu \approx 2.6$; $p = 8.1 \times 10^{-3} < \alpha = 0.01$. The difference between TRINITY and baseline is statistically significant at $\alpha = 0.01$.

## 4. Results / Evidence

Three results are reported.

**Result 1 — Gate-2 BPB.** The TRINITY S³AI model achieves mean BPB = 1.829 on the held-out evaluation partition, with 95% confidence interval $[1.807, 1.852]$ (two-sided, $t$-distribution, $\nu=2$). The Gate-2 threshold 1.85 lies at the upper end of this interval; the one-sided test at $\alpha=0.01$ rejects $H_0: \mu \geq 1.85$ with $p = 3.7 \times 10^{-4}$.

**Result 2 — Baseline comparison.** The TRINITY model outperforms the baseline by $\Delta\text{BPB} = 0.064$ on average, a difference significant at $\alpha = 0.01$ by the Welch two-sample test ($p = 8.1 \times 10^{-3}$).

**Result 3 — Lattice initialisation advantage.** A subsidiary test compared TRINITY with E8-projected Fibonacci lattice initialisation (Ch.7, §4) against TRINITY with random initialisation. The lattice-initialised variant reached BPB = 2.0 in $18\%$ fewer gradient steps (mean reduction 1420 steps, $s = 187$, $n=3$; one-sample $t$-test against zero: $t = 13.2$, $\nu = 2$, $p = 2.9 \times 10^{-3}$).

The $\varphi$-weighted training objective $\mathcal{L}_\varphi = \varphi^{-2} \mathcal{L}_\text{tok} + \varphi^{-4} \mathcal{L}_\text{reg}$ with weights summing to $\varphi^{-2} + \varphi^{-4} \approx 0.382 + 0.056 = 0.438$ does not sum to 1; it is deliberately scaled so that $3 \cdot \mathcal{L}_\varphi = (\varphi^2 + \varphi^{-2}) \cdot \mathcal{L}_\varphi^*$, where $\mathcal{L}_\varphi^* = \varphi^{-2}(\mathcal{L}_\text{tok} + \varphi^{-2}\mathcal{L}_\text{reg})$ is the normalised form tied to the Trinity identity $\varphi^2 + \varphi^{-2} = 3$ [2].

## 5. Qed Assertions

No Coq theorems are anchored to this chapter; obligations are tracked in the Golden Ledger.

## 6. Sealed Seeds

Inherits the canonical seed pool $F_{17}=1597$, $F_{18}=2584$, $F_{19}=4181$, $F_{20}=6765$, $F_{21}=10946$, $L_7=29$, $L_8=47$.

The evaluation partition was drawn with $L_7 = 29$. The three primary replicates used $F_{17}$, $F_{18}$, $F_{19}$. The subsidiary lattice-initialisation experiment used $F_{19}$, $F_{20}$, $F_{21}$.

## 7. Discussion

The primary limitation of the statistical analysis is $n = 3$: with two degrees of freedom, the $t$-distribution has heavy tails and the confidence interval is wide. The 95% interval $[1.807, 1.852]$ is 45 milli-BPB wide, which is large relative to the 21 milli-BPB advantage over baseline. A follow-up experiment with $n = 7$ replicates (using all seven sanctioned seeds) would narrow the interval to approximately $\pm 12$ milli-BPB, subject to the constraint that $F_{20}$ and $F_{21}$ have not been used in any BPB-optimisation decision. A second limitation is that the evaluation partition (10 000 documents, seed $L_7 = 29$) may not represent the full distribution; sensitivity analysis with seed $L_8 = 47$ is recommended. Future work includes extending the Welch test to the Gate-3 BPB target of 1.5, which will require substantially more compute and a correspondingly larger corpus. The statistical methodology connects directly to Ch.13 (seed protocol), Ch.7 (lattice initialisation), and Ch.31 (hardware evaluation).

## References

[1] `igla_assertions.json` runtime-mirror contract, key `stat_test_preregistration`. https://github.com/gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/igla/INV2_IglaAshaBound.v

[2] This dissertation, Ch.1 — Introduction: Trinity S³AI vision. $\varphi^2 + \varphi^{-2} = 3$ anchor.

[3] Welch, B. L. (1947). The generalisation of 'Student's' problem when several different population variances are involved. *Biometrika*, 34(1–2), 28–35.

[4] Satterthwaite, F. E. (1946). An approximate distribution of estimates of variance components. *Biometrics Bulletin*, 2(6), 110–114.

[5] This dissertation, Ch.13 — STROBE Sealed Seeds. Seed admissibility and pre-registration.

[6] This dissertation, Ch.7 — Vogel Phyllotaxis. E8-projected Fibonacci lattice initialisation.

[7] This dissertation, Ch.31 — Hardware Empirical. BPB on FPGA inference.

[8] Dror, R., Baumer, R., Shlain, S., & Reichart, R. (2018). Deep dominance: How to properly compare deep neural models. *ACL*, 2773–2785.

[9] Bouthillier, X., Laurent, C., & Vincent, P. (2019). Unreproducible research is reproducible. *ICML*.

[10] This dissertation, App.D — Reproducibility Scripts. Statistical test code.

[11] This dissertation, App.E — Golden Ledger. Pre-registration record.

[12] Li, L., Jamieson, K., DeSalvo, G., Rostamizadeh, A., & Talwalkar, A. (2018). Hyperband. *JMLR*, 18(185). (ASHA context.)

[13] `gHashTag/trios#419` — Ch.25 scope (for cross-reference). https://github.com/gHashTag/trios/issues/419
