//! Raw FFI declarations for zig-physics C API.

use libc::{c_int, size_t};

/// 3D vector for physics simulations.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Result of a CHSH inequality test.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChshResult {
    /// S parameter (classical bound: |S| ≤ 2, quantum: |S| ≤ 2√2)
    pub s_value: f64,
    /// Whether the inequality is violated
    pub violated: bool,
    /// Correlation coefficient for angle setting A
    pub correlation_a: f64,
    /// Correlation coefficient for angle setting B
    pub correlation_b: f64,
}

/// Golden ratio-derived fundamental constants.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GfConstants {
    pub phi: f64,
    pub fine_structure_approx: f64,
    pub proton_mass_ratio: f64,
    pub coupling_strength: f64,
}

extern "C" {
    /// Compute the CHSH Bell inequality parameter.
    pub fn physics_chsh_bell(
        angle_a1: f64,
        angle_a2: f64,
        angle_b1: f64,
        angle_b2: f64,
    ) -> ChshResult;

    /// Get golden ratio-derived fundamental constants.
    pub fn physics_gf_constants() -> GfConstants;

    /// Simulate a quantum state evolution step.
    /// `state` is a complex vector of length `2*n_qubits`.
    pub fn physics_quantum_step(
        n_qubits: size_t,
        state: *mut f64,
        hamiltonian: *const f64,
        dt: f64,
    ) -> c_int;

    /// Compute gravitational field at a point due to N masses.
    pub fn physics_gravity_field(
        masses: *const f64,
        positions: *const Vec3,
        n_masses: size_t,
        point: Vec3,
        out_field: *mut Vec3,
    ) -> c_int;

    /// Compute QCD coupling constant at given energy scale.
    pub fn physics_qcd_coupling(energy_gev: f64) -> f64;

    /// Compute Fibonacci-based lattice spacing for discrete spacetime.
    pub fn physics_fibonacci_lattice_spacing(level: c_int) -> f64;
}
