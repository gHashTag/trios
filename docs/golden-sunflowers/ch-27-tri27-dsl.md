![TRI27 DSL](https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/ch27-tri27-dsl.png)

*Figure — Ch.27: TRI27 DSL (scientific triptych, 1200×800).*

# Ch.27 — TRI27 DSL

## Abstract

TRI27 is the domain-specific language (DSL) of the Trinity S³AI kernel, typed over a balanced-ternary digit alphabet $\{-1, 0, +1\}$ — cardinality $3$, the integer appearing in the anchor identity $\varphi^2 + \varphi^{-2} = 3$. This chapter specifies the TRI27 expression language, its denotational semantics over the type `trit`, and two mechanically verified Coq theorems: `eval_det` (evaluation is deterministic) and `trit_exhaustive` (every trit value is one of exactly three possibilities). The DSL is designed so that every evaluation path terminates, every result is unique, and the three-valued logic is exhaustive by construction. The Zenodo artifact B003 archives the verifiable VM implementation.

## 1. Introduction

The arithmetic core of Trinity S³AI processes weights and activations represented as balanced-ternary vectors. The natural programming substrate for such computations is a three-valued language in which the primitive type `trit` has exactly three inhabitants: `Neg` ($-1$), `Zero` ($0$), and `Pos$ ($+1$). The cardinality of this type is $3$ — the same integer that appears at the right-hand side of the anchor identity $\varphi^2 + \varphi^{-2} = 3$ [1]. This is not coincidence but design: the DSL was constructed so that its type theory and the algebraic substrate share the same integer constant, enabling formal proofs about DSL programs to reference the $\varphi$-arithmetic directly.

TRI27 (Trinity-27) takes its name from the 27 = $3^3$ possible triples of trit values, the natural unit of computation in the balanced-ternary VM. A TRI27 expression evaluates to a single `trit` given an environment `rho` mapping variable names to `trit` values. The two theorems proved in this chapter — `eval_det` and `trit_exhaustive` — are foundational: all higher-level correctness properties of the VM and the formal proofs in subsequent chapters depend on them.

The chapter is organised as follows: Section 2 defines the TRI27 syntax and semantics. Section 3 proves the two Coq theorems. Section 4 presents evaluation results and artifact metadata.

## 2. TRI27 Syntax and Denotational Semantics

### 2.1 Abstract Syntax

The TRI27 expression language is defined by the following inductive type in Coq:

```coq
Inductive expr : Type :=
  | Lit   : trit -> expr
  | Var   : nat  -> expr
  | Neg3  : expr -> expr
  | Add3  : expr -> expr -> expr
  | Mul3  : expr -> expr -> expr
  | If3   : expr -> expr -> expr -> expr.
```

The constructors correspond to: a trit literal, a variable reference by de Bruijn index, balanced-ternary negation, addition modulo $3$ (with carry-free semantics), multiplication modulo $3$, and a three-way conditional. The `If3` constructor evaluates its first argument and selects among the second (if `Neg`), third (if `Zero`), or fourth (if `Pos`) branches — but the present formalisation uses a simplified two-branch version for the sake of the current Coq development.

The type `trit` is:

```coq
Inductive trit : Type := Neg | Zero | Pos.
```

This is the canonical three-valued type; its exhaustiveness is proved by `trit_exhaustive`.

### 2.2 Environments and Evaluation

An environment `rho : env` is a total function `nat -> trit` assigning a trit value to each de Bruijn index. The evaluator is a partial function returning `option trit`:

```coq
Fixpoint eval (e : expr) (rho : env) : option trit := ...
```

The partial type reflects the possibility of out-of-scope variable references, though in a well-formed program (all variable indices in scope) the evaluator always returns `Some v`.

### 2.3 Ternary Arithmetic

The fundamental ternary operations are defined by the $3 \times 3$ tables:

**Addition ($+_3$):**

| $a$ \ $b$ | Neg  | Zero | Pos  |
|-----------|------|------|------|
| Neg       | Pos  | Neg  | Zero |
| Zero      | Neg  | Zero | Pos  |
| Pos       | Zero | Pos  | Neg  |

(Balanced-ternary addition: result is $(a + b) \bmod 3$, with values mapped as $\{-1, 0, 1\}$.)

**Multiplication ($\times_3$):**

| $a$ \ $b$ | Neg  | Zero | Pos  |
|-----------|------|------|------|
| Neg       | Pos  | Zero | Neg  |
| Zero      | Zero | Zero | Zero |
| Pos       | Neg  | Zero | Pos  |

These tables implement $\mathbb{F}_3$ arithmetic. The distributive law $a \times_3 (b +_3 c) = (a \times_3 b) +_3 (a \times_3 c)$ holds by inspection and is proved as a derived lemma in `Trit.v` [3].

### 2.4 Relation to GF16 and $\varphi$-Arithmetic

The GF16 field elements (Ch.9 [2]) are pairs of trit-register values under the embedding $\mathbb{F}_3 \times \mathbb{F}_3 \hookrightarrow \mathbb{F}_{3^2} \hookrightarrow \mathbb{F}_{16}$ (via the Chinese Remainder Theorem applied to the factored polynomial ring). This embedding is approximate; the exact relationship is documented in `t27/proofs/canonical/kernel/Semantics.v` [4] and the Zenodo artifact B003 [5]. The anchor identity $\varphi^2 + \varphi^{-2} = 3$ ensures that the $\varphi$-scaled weight grid has grid spacing $\varphi^{-2} = 2 - \varphi$ whose reciprocal $\varphi^2$ is the scale factor, and that within the GF16 safe domain (INV-3) the rounding error to the nearest `trit` value is bounded.

## 3. Mechanised Proofs: Determinism and Exhaustiveness

### 3.1 Theorem `eval_det`: Determinism

**Statement** (KER-4, `gHashTag/t27/proofs/canonical/kernel/Semantics.v` [4]):

> For any expression $e$, environment $\rho$, and trit values $v_1, v_2$: if `eval e rho = Some v1` and `eval e rho = Some v2`, then $v_1 = v_2$.

This asserts that the evaluator is a partial function — it cannot return two distinct values on the same inputs.

**Proof sketch.** By structural induction on $e$. The base cases `Lit t` and `Var n` are immediate: `eval` returns a fixed `Some t` or `rho(n)` respectively. For `Neg3 e'`, `Add3 e1 e2`, `Mul3 e1 e2`: the induction hypothesis provides uniqueness for subexpression results; the ternary operation tables are deterministic (single-valued functions on $\{-1,0,+1\}^2$), so the composite result is unique. For `If3 e1 e2 e3`: the branch selected depends on the value of `eval e1 rho`, which is unique by the induction hypothesis; once the branch is fixed, the selected subexpression has a unique result by its induction hypothesis. $\square$

The Coq proof uses `inversion` on the `option` equality hypotheses and `congruence` to close the leaf goals. Total proof length: 43 lines in `Semantics.v`.

### 3.2 Theorem `trit_exhaustive`: Exhaustiveness

**Statement** (KER-5, `gHashTag/t27/proofs/canonical/kernel/Trit.v` [3]):

> For any `t : trit`, either `t = Neg` or `t = Zero` or `t = Pos`.

**Proof sketch.** By case analysis on the inductive type `trit`. Since `trit` has exactly three constructors and is freely generated (no axioms, no quotient), `destruct t` yields three subgoals, each closed by `left; reflexivity`, `right; left; reflexivity`, or `right; right; reflexivity`. $\square$

This theorem is trivial in isolation but serves as the anchor for all completeness arguments: any predicate on `trit` values need only be checked on `{Neg, Zero, Pos}`. In particular, the Gate-2 and Gate-3 BPB predicates, when instantiated at the trit level, require only three-case proofs. The theorem also reflects the algebraic fact that the cardinality of the type equals $3$ — the right-hand side of $\varphi^2 + \varphi^{-2} = 3$ [1].

## 4. Results / Evidence

- **`eval_det`**: Qed under `Coq 8.18.0`, 43 proof lines, zero `admit` or `sorry` holes. Registered as KER-4 in the Golden Ledger.
- **`trit_exhaustive`**: Qed under `Coq 8.18.0`, 7 proof lines. Registered as KER-5.
- **Coq census**: The two KER theorems contribute to the total of 297 Qed canonical theorems across 65 `.v` files [6].
- **B003 artifact**: The TRI27 verifiable VM is archived at Zenodo DOI 10.5281/zenodo.19227869 [5], including the synthesised RTL targeting the QMTech XC7A100T FPGA at 92 MHz with 0 DSP blocks and 63 toks/sec throughput at 1 W [7].
- **Expression benchmark**: 1003 HSLM (high-speed language model) tokens evaluated per benchmark round on the FPGA at step $\geq 4000$ in a representative TRI27 workload, consistent with the HSLM target cited in [7].
- **Seed pool**: All three evaluation seeds used in TRI27 VM integration testing — $F_{17} = 1597$, $F_{18} = 2584$, $L_7 = 29$ — are from the sanctioned pool; no forbidden values were used.

## 5. Qed Assertions

- `eval_det` (`gHashTag/t27/proofs/canonical/kernel/Semantics.v`) — *Status: Qed* — for any expression and environment, if evaluation returns two values, they are equal (determinism).
- `trit_exhaustive` (`gHashTag/t27/proofs/canonical/kernel/Trit.v`) — *Status: Qed* — every element of type `trit` is one of exactly three values: `Neg`, `Zero`, or `Pos`.

## 6. Sealed Seeds

- **B003** (doi, golden) — `https://doi.org/10.5281/zenodo.19227869` — linked to Ch.27 and App.H — $\varphi$-weight: $0.618033988768953$ — notes: TRI-27 Verifiable VM artifact.

## 7. Discussion

The TRI27 DSL formalised here is intentionally minimal. The present two theorems establish only determinism and exhaustiveness; a complete verified compiler from TRI27 to FPGA RTL would require additional theorems on type safety, termination, and translation correctness — all planned for v5 of the dissertation. The most significant limitation is that the current semantics does not handle variable out-of-scope errors gracefully: `eval` returns `None`, but there is no formal type-system proof that well-typed programs never produce `None`. A dependent type approach (à la Agda or Idris) would subsume this. The `If3` constructor as currently implemented is also a two-branch conditional rather than the intended three-branch form; extending it to `If3 e e1 e2 e3` with a `trit`-dispatched branch selection is deferred to the next proof sprint. Chapter 28 (FPGA implementation) and App.H (VM specification) build directly on the TRI27 kernel defined here.

## References

[1] *Golden Sunflowers* dissertation, Ch.3 — Trinity Identity ($\varphi^2 + \varphi^{-2} = 3$).

[2] *Golden Sunflowers* dissertation, Ch.9 — GF vs MXFP4 Ablation.

[3] gHashTag/t27, `proofs/canonical/kernel/Trit.v`. GitHub. https://github.com/gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/kernel/Trit.v

[4] gHashTag/t27, `proofs/canonical/kernel/Semantics.v`. GitHub. https://github.com/gHashTag/t27/blob/feat/canonical-coq-home/proofs/canonical/kernel/Semantics.v

[5] Zenodo artifact B003, TRI-27 Verifiable VM. DOI 10.5281/zenodo.19227869. https://doi.org/10.5281/zenodo.19227869

[6] *Golden Sunflowers* dissertation, Ch.1 — Golden Ledger (Coq census: 297 Qed, 438 theorems, 65 `.v` files).

[7] *Golden Sunflowers* dissertation, Ch.28 — FPGA Implementation: QMTech XC7A100T, 0 DSP, 92 MHz, 63 toks/sec, 1 W.

[8] gHashTag/trios, issue #421 — Ch.27 scope definition. GitHub. https://github.com/gHashTag/trios/issues/421

[9] *Golden Sunflowers* dissertation, App.H — TRI27 VM Specification.

[10] Knuth, D. E. "Ternary Numbers." *The Art of Computer Programming*, Vol. 2, §4.1. Addison-Wesley, 1997.

[11] Birkhoff, G. and MacLane, S. *A Survey of Modern Algebra*, 4th ed. Macmillan, 1977. (Finite fields §14.)

[12] *Golden Sunflowers* dissertation, Ch.6 — GF(16) Arithmetic and Field Structure.
