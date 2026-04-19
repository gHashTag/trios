//! # trios-physics
//!
//! Safe Rust wrapper around [zig-physics](https://github.com/gHashTag/zig-physics),
//! providing quantum mechanics, QCD, and gravity simulation primitives.
//!
//! ## Example
//!
//! ```ignore
//! use trios_physics::{chsh_bell, gf_constants};
//!
//! let result = chsh_bell(0.0, std::f64::consts::PI / 4.0, std::f64::consts::PI / 8.0, 3.0 * std::f64::consts::PI / 8.0);
//! assert!(result.s_value > 2.0, "quantum violation!");
//!
//! let constants = gf_constants();
//! println!("φ = {}", constants.phi);
//! ```

mod ffi;

pub use ffi::{ChshResult, GfConstants, Vec3};

/// Run a CHSH Bell inequality test with given measurement angles (in radians).
///
/// Returns a [`ChshResult`] with the S-parameter and violation status.
/// Classical bound: |S| ≤ 2. Quantum bound: |S| ≤ 2√2 ≈ 2.828.
pub fn chsh_bell(angle_a1: f64, angle_a2: f64, angle_b1: f64, angle_b2: f64) -> ChshResult {
    unsafe { ffi::physics_chsh_bell(angle_a1, angle_a2, angle_b1, angle_b2) }
}

/// Get golden ratio-derived fundamental constants.
pub fn gf_constants() -> GfConstants {
    unsafe { ffi::physics_gf_constants() }
}

/// Simulate one step of quantum state evolution under a Hamiltonian.
///
/// `state` is a complex vector (real/imaginary interleaved) of length `2 * 2^n_qubits`.
/// `hamiltonian` is the Hamiltonian matrix (same layout).
/// `dt` is the time step.
pub fn quantum_step(n_qubits: usize, state: &mut [f64], hamiltonian: &[f64], dt: f64) -> Result<(), String> {
    let expected_len = 2 * (1 << n_qubits);
    if state.len() != expected_len || hamiltonian.len() != expected_len * expected_len {
        return Err(format!(
            "dimension mismatch: expected state={}, hamiltonian={}, got state={}, hamiltonian={}",
            expected_len,
            expected_len * expected_len,
            state.len(),
            hamiltonian.len()
        ));
    }
    let rc = unsafe {
        ffi::physics_quantum_step(n_qubits, state.as_mut_ptr(), hamiltonian.as_ptr(), dt)
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(format!("quantum_step failed with code {rc}"))
    }
}

/// Compute gravitational field at a point due to N point masses.
///
/// - `masses`: slice of mass values (kg)
/// - `positions`: slice of [`Vec3`] positions for each mass
/// - `point`: the point at which to evaluate the field
///
/// Returns the gravitational field vector at `point`.
pub fn gravity_field(masses: &[f64], positions: &[Vec3], point: Vec3) -> Result<Vec3, String> {
    if masses.len() != positions.len() {
        return Err("masses and positions must have the same length".into());
    }
    let mut out = Vec3::default();
    let rc = unsafe {
        ffi::physics_gravity_field(
            masses.as_ptr(),
            positions.as_ptr(),
            masses.len(),
            point,
            &mut out,
        )
    };
    if rc == 0 {
        Ok(out)
    } else {
        Err(format!("gravity_field failed with code {rc}"))
    }
}

/// Compute QCD coupling constant at given energy scale (in GeV).
pub fn qcd_coupling(energy_gev: f64) -> f64 {
    unsafe { ffi::physics_qcd_coupling(energy_gev) }
}

/// Compute Fibonacci-based lattice spacing for discrete spacetime at given level.
pub fn fibonacci_lattice_spacing(level: i32) -> f64 {
    unsafe { ffi::physics_fibonacci_lattice_spacing(level) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires zig-physics vendor submodule"]
    fn chsh_quantum_violation() {
        let result = chsh_bell(0.0, std::f64::consts::PI / 4.0, std::f64::consts::PI / 8.0, 3.0 * std::f64::consts::PI / 8.0);
        assert!(result.violated, "CHSH should be violated for quantum angles");
        assert!(result.s_value > 2.0, "S should exceed classical bound of 2");
    }

    #[test]
    #[ignore = "requires zig-physics vendor submodule"]
    fn gf_constants_phi() {
        let c = gf_constants();
        assert!((c.phi - 1.6180339887).abs() < 1e-9, "φ should be golden ratio");
    }
}
