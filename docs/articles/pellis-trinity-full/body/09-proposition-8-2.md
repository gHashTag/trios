## Proposition 8.2 — convention fix

Earlier drafts of Proposition 8.2 carried an internal convention conflict:
the statement asserted $\mu_T(x) = 0$ while the proof derived
$\mu_T(x) = -\infty$. This article resolves the conflict by separating
the two quantities explicitly.

### Definitions in force

Let $R(x)$ denote the **residual** of the approximation at $x$, and let
$\mu_T(x)$ denote the **approximation exponent** governing the decay of
the Pellis–Trinity approximation kernel.

**Convention (Z).** *Zero residual.* If the approximation is exact at $x$
in the sense that no further error remains, we write

$$
R(x) = 0.
$$

**Convention (E).** *Approximation exponent at exactness.* By the
exponential parameterisation of the kernel, exactness implies the formal
exponent

$$
\mu_T(x) = -\infty,
$$

with the standard convention $e^{-\infty} = 0$.

### Proposition 8.2 (corrected statement)

If the Pellis–Trinity approximation is exact at $x$, then the residual
satisfies $R(x) = 0$ **and** equivalently the approximation exponent
satisfies $\mu_T(x) = -\infty$.

The proof now derives the exponent statement under Convention (E) without
contradicting the residual statement under Convention (Z). The two are
distinct quantities and the earlier statement of "$\mu_T(x) = 0$ at
exactness" is withdrawn.
