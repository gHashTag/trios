//! HD-01 — HDC operations (bind, bundle, similarity)

use trios_hdc_hd00::Hypervector;

/// Bind two hypervectors (element-wise multiplication for bipolar).
pub fn bind(a: &Hypervector, b: &Hypervector) -> Hypervector {
    assert_eq!(a.dim(), b.dim());
    let v: Vec<i8> = a.0.iter().zip(b.0.iter()).map(|(x, y)| x * y).collect();
    Hypervector(v)
}

/// Bundle (sum) — element-wise sign of sum.
pub fn bundle(a: &Hypervector, b: &Hypervector) -> Hypervector {
    assert_eq!(a.dim(), b.dim());
    let v: Vec<i8> = a
        .0
        .iter()
        .zip(b.0.iter())
        .map(|(x, y)| {
            let s = (*x as i32) + (*y as i32);
            if s > 0 {
                1
            } else if s < 0 {
                -1
            } else {
                0
            }
        })
        .collect();
    Hypervector(v)
}

/// Cosine-like similarity: dot product / dim.
pub fn similarity(a: &Hypervector, b: &Hypervector) -> f64 {
    assert_eq!(a.dim(), b.dim());
    let dot: i64 = a.0.iter().zip(b.0.iter()).map(|(x, y)| (*x as i64) * (*y as i64)).sum();
    dot as f64 / a.dim() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_self_is_ones() {
        let a = Hypervector(vec![1, -1, 1, -1]);
        let r = bind(&a, &a);
        assert_eq!(r, Hypervector(vec![1, 1, 1, 1]));
    }

    #[test]
    fn similarity_self_is_one() {
        let a = Hypervector(vec![1, -1, 1, -1]);
        assert_eq!(similarity(&a, &a), 1.0);
    }

    #[test]
    fn bundle_works() {
        let a = Hypervector(vec![1, 1, -1]);
        let b = Hypervector(vec![1, -1, -1]);
        assert_eq!(bundle(&a, &b), Hypervector(vec![1, 0, -1]));
    }
}
