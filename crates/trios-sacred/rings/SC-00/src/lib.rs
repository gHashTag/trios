//! SC-00 — sacred geometry primitives
//!
//! Bottom of the ring graph for trios-sacred.

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn dot(self, other: Vec2) -> f64 {
        self.x * other.x + self.y * other.y
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Triangle(pub [Vec2; 3]);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2_dot() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert_eq!(a.dot(b), 0.0);
    }
}
