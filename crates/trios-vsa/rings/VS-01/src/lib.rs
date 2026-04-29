//! VS-01 — VSA binding (bind/unbind/superpose)

use trios_vsa_vs00::Symbol;

/// Bind two symbols (bipolar element-wise multiplication).
pub fn bind(a: &Symbol, b: &Symbol) -> Symbol {
    assert_eq!(a.dim(), b.dim());
    Symbol(a.0.iter().zip(b.0.iter()).map(|(x, y)| x * y).collect())
}

/// Unbind — for bipolar, equal to bind (involution).
pub fn unbind(bound: &Symbol, key: &Symbol) -> Symbol {
    bind(bound, key)
}

/// Superpose — element-wise sign-of-sum.
pub fn superpose(a: &Symbol, b: &Symbol) -> Symbol {
    assert_eq!(a.dim(), b.dim());
    Symbol(
        a.0.iter()
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
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_unbind_recovers() {
        let key = Symbol(vec![1, -1, 1, -1]);
        let val = Symbol(vec![1, 1, -1, -1]);
        let bound = bind(&key, &val);
        let recovered = unbind(&bound, &key);
        assert_eq!(recovered, val);
    }

    #[test]
    fn superpose_works() {
        let a = Symbol(vec![1, 1, -1]);
        let b = Symbol(vec![1, -1, -1]);
        assert_eq!(superpose(&a, &b), Symbol(vec![1, 0, -1]));
    }
}
