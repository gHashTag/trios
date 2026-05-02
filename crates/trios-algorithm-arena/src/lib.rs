//! trios-algorithm-arena — GOLD II crate facade.
//!
//! Re-exports SR-ALG-00 typed primitives for every downstream GOLD II
//! ring (SR-ALG-01 jepa, SR-ALG-02 universal-transformer, SR-ALG-03
//! e2e-ttt, BR-OUTPUT AlgorithmArena assembler).
//!
//! L-RING-FACADE-001: this file MUST NOT contain business logic — only
//! re-exports.

pub use trios_algorithm_arena_sr_alg_00::*;
