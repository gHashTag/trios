//! # trios-physics
//!
//! Safe Rust wrapper around zig-physics, providing quantum mechanics, QCD, and gravity simulation.
//! When Zig vendor is not available, all FFI-dependent functions return stubs.

mod ffi;
pub use ffi::{ChshResult, GfConstants, Vec3};

#[cfg(has_zig_lib)]
pub fn chsh_bell(angle_a1: f64, angle_a2: f64, angle_b1: f64, angle_b2: f64) -> ChshResult {
    unsafe { ffi::physics_chsh_bell(angle_a1, angle_a2, angle_b1, angle_b2) }
}
#[cfg(not(has_zig_lib))]
pub fn chsh_bell(_angle_a1: f64, _angle_a2: f64, _angle_b1: f64, _angle_b2: f64) -> ChshResult {
    ChshResult {
        s_value: 0.0,
        violated: false,
        correlation_a: 0.0,
        correlation_b: 0.0,
    }
}

#[cfg(has_zig_lib)]
pub fn gf_constants() -> GfConstants {
    unsafe { ffi::physics_gf_constants() }
}
#[cfg(not(has_zig_lib))]
pub fn gf_constants() -> GfConstants {
    GfConstants {
        phi: 1.618033988749895,
        fine_structure_approx: 0.0,
        proton_mass_ratio: 0.0,
        coupling_strength: 0.0,
    }
}

#[cfg(has_zig_lib)]
pub fn quantum_step(
    n_qubits: usize,
    state: &mut [f64],
    hamiltonian: &[f64],
    dt: f64,
) -> Result<(), String> {
    let expected_len = 2 * (1 << n_qubits);
    if state.len() != expected_len || hamiltonian.len() != expected_len * expected_len {
        return Err("dimension mismatch".to_string());
    }
    let rc = unsafe {
        ffi::physics_quantum_step(n_qubits, state.as_mut_ptr(), hamiltonian.as_ptr(), dt)
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(format!("quantum_step failed with code {}", rc))
    }
}
#[cfg(not(has_zig_lib))]
pub fn quantum_step(
    _n_qubits: usize,
    _state: &mut [f64],
    _hamiltonian: &[f64],
    _dt: f64,
) -> Result<(), String> {
    Err("zig-physics FFI not available".into())
}

#[cfg(has_zig_lib)]
pub fn gravity_field(masses: &[f64], positions: &[Vec3], point: Vec3) -> Result<Vec3, String> {
    if masses.len() != positions.len() {
        return Err("mismatch".into());
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
#[cfg(not(has_zig_lib))]
pub fn gravity_field(_masses: &[f64], _positions: &[Vec3], _point: Vec3) -> Result<Vec3, String> {
    Err("zig-physics FFI not available".into())
}

#[cfg(has_zig_lib)]
pub fn qcd_coupling(energy_gev: f64) -> f64 {
    unsafe { ffi::physics_qcd_coupling(energy_gev) }
}
#[cfg(not(has_zig_lib))]
pub fn qcd_coupling(_energy_gev: f64) -> f64 {
    0.0
}

#[cfg(has_zig_lib)]
pub fn fibonacci_lattice_spacing(level: i32) -> f64 {
    unsafe { ffi::physics_fibonacci_lattice_spacing(level) }
}
#[cfg(not(has_zig_lib))]
pub fn fibonacci_lattice_spacing(_level: i32) -> f64 {
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf_constants_phi_stub() {
        let c = gf_constants();
        assert!((c.phi - 1.618033988749895).abs() < 1e-9);
    }

    #[test]
    fn chsh_stub() {
        let r = chsh_bell(0.0, 0.0, 0.0, 0.0);
        if !cfg!(has_zig_lib) {
            assert!(!r.violated);
        }
    }
}
