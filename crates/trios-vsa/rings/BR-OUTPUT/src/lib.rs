//! BR-OUTPUT — trios-vsa assembly

pub use trios_vsa_vs00::Symbol;
pub use trios_vsa_vs01::{bind, superpose, unbind};

pub struct Vsa;

impl Vsa {
    pub const fn anchor() -> f64 {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_link_bind_unbind() {
        let key = Symbol(vec![1, -1, 1]);
        let val = Symbol(vec![1, 1, -1]);
        let b = bind(&key, &val);
        assert_eq!(unbind(&b, &key), val);
    }
}
