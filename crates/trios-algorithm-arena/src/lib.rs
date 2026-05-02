//! trios-algorithm-arena — GOLD II crate facade.
//!
//! Re-exports SR-ALG-00 typed primitives for every downstream GOLD II
//! ring (SR-ALG-01 jepa, SR-ALG-02 universal-transformer, SR-ALG-03
//! e2e-ttt, BR-OUTPUT AlgorithmArena assembler).
//!
//! L-RING-FACADE-001: this file MUST NOT contain business logic — only
//! re-exports.

pub use trios_algorithm_arena_sr_alg_00::*;
pub use trios_algorithm_arena_sr_alg_03::{
    build_spec, evaluate_gate, sha256_bytes, E2eTttEnv, GateVerdict, Verifier,
    VerifierOutcome, COQ_THEOREM_ID, EMBARGO_DAYS, ENTRY_PATH, FIBONACCI_SEEDS,
    TARGET_VAL_BPB,
};
pub use trios_algorithm_arena_br_output::{
    AlgorithmArena, ArenaError, BpbRow, BpbRowId, MockedTrainer, RunBackend, TrainerBackend,
};
