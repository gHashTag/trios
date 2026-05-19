// SPDX-License-Identifier: Apache-2.0
// Trinity Loss — φ-prior-aware ternary contrastive loss for JEPA-T
// Author: Dmitrii Vasilev <admin@t27.ai>
//
// Formula:
//   sim(a,b)      = dot_ternary(a,b) / 64
//   L_triplet     = max(0, margin + sim(a,n) - sim(a,p))
//   L_phi_prior   = phi_inv_sq * (zero_count(a) + zero_count(p) + zero_count(n)) / 192
//   L_total       = L_triplet + lambda * L_phi_prior
//
// where phi_inv_sq = φ⁻² ≈ 0.382, margin = 0.5, lambda = 0.1.
//
// All functions are deterministic, allocation-free (no heap), and use no std::time.

/// φ⁻² = 1/(φ²) ≈ 0.381966...  — truncated to 3 significant figures per spec.
pub const PHI_INV_SQ: f32 = 0.382;

/// Default margin for the triplet loss (φ⁻² ≈ 0.5 is the spec value).
pub const DEFAULT_MARGIN: f32 = 0.5;

/// Default λ weighting for the φ-prior term.
pub const DEFAULT_LAMBDA: f32 = 0.1;

/// Compute the ternary dot product of two 64-element ternary vectors.
///
/// Elements are expected to be in {-1, 0, 1}; the function is defined for
/// all i8 values but is meaningful only for ternary inputs.
///
/// # Examples
/// ```
/// use trinity_loss::dot_ternary;
/// let a = [1i8; 64];
/// let b = [1i8; 64];
/// assert_eq!(dot_ternary(&a, &b), 64);
/// ```
#[inline]
pub fn dot_ternary(a: &[i8; 64], b: &[i8; 64]) -> i32 {
    let mut acc: i32 = 0;
    let mut i = 0;
    while i < 64 {
        acc += (a[i] as i32) * (b[i] as i32);
        i += 1;
    }
    acc
}

/// Ternary cosine analogue: `dot_ternary(a, b) / 64`.
///
/// Range is [-1.0, 1.0] for unit ternary vectors; can exceed ±1 for dense
/// vectors with only non-zero entries (maximum = 1.0, minimum = -1.0 for
/// properly ternary {-1,0,1} inputs).
///
/// # Examples
/// ```
/// use trinity_loss::sim;
/// let a = [1i8; 64];
/// let b = [-1i8; 64];
/// assert_eq!(sim(&a, &b), -1.0_f32);
/// ```
#[inline]
pub fn sim(a: &[i8; 64], b: &[i8; 64]) -> f32 {
    dot_ternary(a, b) as f32 / 64.0
}

/// Count the number of zero entries in a ternary vector (ℓ₀ norm of zeros).
///
/// # Examples
/// ```
/// use trinity_loss::zero_count;
/// let a = [0i8; 64];
/// assert_eq!(zero_count(&a), 64);
/// ```
#[inline]
pub fn zero_count(a: &[i8; 64]) -> u32 {
    let mut cnt: u32 = 0;
    let mut i = 0;
    while i < 64 {
        if a[i] == 0 {
            cnt += 1;
        }
        i += 1;
    }
    cnt
}

/// The φ-prior sparsity penalty term (scalar).
///
/// ```text
/// L_phi_prior = PHI_INV_SQ * (zero_count(a) + zero_count(p) + zero_count(n)) / 192
/// ```
///
/// 192 = 3 × 64 normalises by the total number of entries across the triplet.
///
/// # Examples
/// ```
/// use trinity_loss::phi_prior_term;
/// let a = [0i8; 64];
/// let p = [0i8; 64];
/// let n = [0i8; 64];
/// // All zeros: phi_prior = 0.382 * 192 / 192 = 0.382
/// let expected = 0.382_f32;
/// assert!((phi_prior_term(&a, &p, &n) - expected).abs() < 1e-4);
/// ```
#[inline]
pub fn phi_prior_term(a: &[i8; 64], p: &[i8; 64], n: &[i8; 64]) -> f32 {
    let zeros = zero_count(a) + zero_count(p) + zero_count(n);
    PHI_INV_SQ * (zeros as f32) / 192.0
}

/// Compute the full Trinity loss for one (anchor, positive, negative) triplet.
///
/// ```text
/// L_triplet  = max(0, margin + sim(a,n) - sim(a,p))
/// L_phi      = PHI_INV_SQ * (zero_count(a)+zero_count(p)+zero_count(n)) / 192
/// L_total    = L_triplet + lambda * L_phi
/// ```
///
/// # Arguments
/// * `a`      – anchor ternary vector
/// * `p`      – positive ternary vector (semantically similar to anchor)
/// * `n`      – negative ternary vector (semantically dissimilar from anchor)
/// * `margin` – triplet margin (default 0.5 = `DEFAULT_MARGIN`)
/// * `lambda` – φ-prior weighting (default 0.1 = `DEFAULT_LAMBDA`)
///
/// # Examples
/// ```
/// use trinity_loss::{trinity_loss, DEFAULT_MARGIN, DEFAULT_LAMBDA};
/// let a = [1i8; 64];
/// let p = [1i8; 64];
/// let n = [-1i8; 64];
/// // Perfect triplet: sim(a,p)=1, sim(a,n)=-1 → L_triplet=0, no zeros → L_total=0
/// assert_eq!(trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA), 0.0_f32);
/// ```
#[inline]
pub fn trinity_loss(a: &[i8; 64], p: &[i8; 64], n: &[i8; 64], margin: f32, lambda: f32) -> f32 {
    let sim_ap = sim(a, p);
    let sim_an = sim(a, n);
    let l_triplet = (margin + sim_an - sim_ap).max(0.0);
    let l_phi = phi_prior_term(a, p, n);
    l_triplet + lambda * l_phi
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn dot_all_ones() {
        let a = [1i8; 64];
        let b = [1i8; 64];
        assert_eq!(dot_ternary(&a, &b), 64);
    }

    #[test]
    fn dot_opposite() {
        let a = [1i8; 64];
        let b = [-1i8; 64];
        assert_eq!(dot_ternary(&a, &b), -64);
    }

    #[test]
    fn sim_range() {
        let a = [1i8; 64];
        let b = [1i8; 64];
        assert!((sim(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn zero_count_full() {
        let a = [0i8; 64];
        assert_eq!(zero_count(&a), 64);
    }

    #[test]
    fn zero_count_none() {
        let a = [1i8; 64];
        assert_eq!(zero_count(&a), 0);
    }
}
