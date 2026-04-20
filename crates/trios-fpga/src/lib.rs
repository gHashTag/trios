mod board;
mod build;
mod flash;
mod synth_readiness;
mod xdc;

pub use board::{BoardProfile, KnownBoard};
pub use build::{BuildConfig, BuildPipeline};
pub use flash::{FlashConfig, FlashPipeline};
pub use synth_readiness::SynthReadiness;
pub use xdc::{XdcConstraints, XdcEntry};

pub const FPGA_MODULES: &[&str] = &[
    "mac", "uart", "spi", "bridge", "top_level",
    "hir", "hw_types", "memory", "clock_domain", "fifo",
    "axi4", "apb_bridge", "gf16_accel", "formal",
    "ternary_isa", "stdlib", "simulator", "assembler", "testbench", "vcd_trace",
    "e2e_demo", "linker", "timing", "power", "placement", "partition",
    "router", "dft", "cts", "crossopt", "bootrom",
    "sv_emit", "firrtl", "cdc", "lint", "coverage",
];
