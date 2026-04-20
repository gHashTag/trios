#[cfg(has_zig_lib)]
use libc::{c_int, size_t};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChshResult {
    pub s_value: f64,
    pub violated: bool,
    pub correlation_a: f64,
    pub correlation_b: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfConstants {
    pub phi: f64,
    pub fine_structure_approx: f64,
    pub proton_mass_ratio: f64,
    pub coupling_strength: f64,
}

#[cfg(has_zig_lib)]
extern "C" {
    pub fn physics_chsh_bell(
        angle_a1: f64,
        angle_a2: f64,
        angle_b1: f64,
        angle_b2: f64,
    ) -> ChshResult;
    pub fn physics_gf_constants() -> GfConstants;
    pub fn physics_quantum_step(
        n_qubits: size_t,
        state: *mut f64,
        hamiltonian: *const f64,
        dt: f64,
    ) -> c_int;
    pub fn physics_gravity_field(
        masses: *const f64,
        positions: *const Vec3,
        n_masses: size_t,
        point: Vec3,
        out_field: *mut Vec3,
    ) -> c_int;
    pub fn physics_qcd_coupling(energy_gev: f64) -> f64;
    pub fn physics_fibonacci_lattice_spacing(level: c_int) -> f64;
}
