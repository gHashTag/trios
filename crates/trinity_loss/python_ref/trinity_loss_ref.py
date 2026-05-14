#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Trinity Loss — Golden Python Reference (informational only)
# Author: Dmitrii Vasilev <admin@t27.ai>
#
# This file is a reference implementation matching the Rust crate
# `trinity_loss`. It is NOT part of the cargo build or cargo test suite.
# Used for independent numerical verification of the Rust implementation.
#
# Formula:
#   sim(a,b)     = dot(a,b) / 64               # ternary cosine analogue
#   L_triplet    = max(0, margin + sim(a,n) - sim(a,p))
#   L_phi_prior  = phi_inv_sq * (||a||₀ + ||p||₀ + ||n||₀) / 192
#   L_total      = L_triplet + lambda * L_phi_prior
#
# where ||x||₀ counts zero entries, phi_inv_sq = φ⁻² ≈ 0.382.

from __future__ import annotations
import numpy as np
from typing import Sequence

# Constants (mirror of src/lib.rs)
PHI_INV_SQ: float = 0.382
DEFAULT_MARGIN: float = 0.5
DEFAULT_LAMBDA: float = 0.1


def dot_ternary(a: Sequence[int], b: Sequence[int]) -> int:
    """Ternary dot product of two length-64 ternary vectors."""
    a = np.asarray(a, dtype=np.int32)
    b = np.asarray(b, dtype=np.int32)
    assert len(a) == 64 and len(b) == 64, "Vectors must have length 64"
    return int(np.dot(a, b))


def sim(a: Sequence[int], b: Sequence[int]) -> float:
    """Ternary similarity: dot_ternary(a, b) / 64."""
    return dot_ternary(a, b) / 64.0


def zero_count(a: Sequence[int]) -> int:
    """Count of zero entries in a ternary vector (ℓ₀-zero norm)."""
    return int(np.sum(np.asarray(a) == 0))


def phi_prior_term(
    a: Sequence[int],
    p: Sequence[int],
    n: Sequence[int],
) -> float:
    """φ-prior sparsity penalty: PHI_INV_SQ * total_zeros / 192."""
    total_zeros = zero_count(a) + zero_count(p) + zero_count(n)
    return PHI_INV_SQ * total_zeros / 192.0


def trinity_loss(
    a: Sequence[int],
    p: Sequence[int],
    n: Sequence[int],
    margin: float = DEFAULT_MARGIN,
    lam: float = DEFAULT_LAMBDA,
) -> float:
    """Full Trinity loss for one (anchor, positive, negative) triplet.

    Args:
        a:      anchor ternary vector (length 64, elements in {-1, 0, 1})
        p:      positive ternary vector (semantically similar to a)
        n:      negative ternary vector (semantically dissimilar from a)
        margin: triplet margin (default 0.5)
        lam:    λ weighting for φ-prior term (default 0.1)

    Returns:
        Scalar loss value ≥ 0.
    """
    sim_ap = sim(a, p)
    sim_an = sim(a, n)
    l_triplet = max(0.0, margin + sim_an - sim_ap)
    l_phi = phi_prior_term(a, p, n)
    return l_triplet + lam * l_phi


# ── Verification against the 10 hand-computed test cases ────────────────────

def _run_verification() -> None:
    import math

    cases = [
        # (label, a, p, n, expected_loss)
        ("T01", [1]*64,  [1]*64,  [-1]*64,  0.0),
        ("T02", [0]*64,  [0]*64,  [0]*64,   0.5 + 0.1 * (0.382 * 192 / 192)),
        ("T03",
         ([1,0,-1]*21+[1]),
         ([1,0,-1]*21+[1]),
         ([-1,0,1]*21+[-1]),
         0.0 + 0.1 * (0.382 * 63 / 192)),
        ("T04",
         [1]*32+[-1]*32,  [-1]*64,  [1]*64,  0.5),
        ("T05",
         [0]*32+[1]*32,  [1]*32+[0]*32,  [-1]*64,
         0.0 + 0.1 * (0.382 * 64 / 192)),
        ("T06",
         [1,-1]*32,  [1,-1]*32,  [-1,1]*32,  0.0),
        ("T07",
         [0]*64,  [1]*32+[-1]*32,  [1]*32+[-1]*32,
         0.5 + 0.1 * (0.382 * 64 / 192)),
        ("T08",
         [1]*16+[0]*16+[-1]*16+[0]*16,
         [1]*16+[0]*16+[-1]*16+[0]*16,
         [-1]*16+[0]*16+[1]*16+[0]*16,
         0.0 + 0.1 * (0.382 * 96 / 192)),
        ("T09",
         [1]*64,  [-1]*32+[0]*32,  [1]*32+[0]*32,
         1.5 + 0.1 * (0.382 * 64 / 192)),
        ("T10",
         [0]*48+[1]*16,  [0]*48+[1]*16,  [0]*48+[-1]*16,
         0.0 + 0.1 * (0.382 * 144 / 192)),
    ]

    tol = 1e-4
    all_pass = True
    for label, a, p, n, expected in cases:
        got = trinity_loss(a, p, n)
        ok = math.isclose(got, expected, abs_tol=tol)
        status = "PASS" if ok else "FAIL"
        if not ok:
            all_pass = False
        print(f"[{status}] {label}: got={got:.7f}  expected={expected:.7f}  diff={abs(got-expected):.2e}")

    if all_pass:
        print("\nAll 10 hand-computed cases PASSED.")
    else:
        print("\nSome cases FAILED!")
        raise SystemExit(1)


if __name__ == "__main__":
    _run_verification()
