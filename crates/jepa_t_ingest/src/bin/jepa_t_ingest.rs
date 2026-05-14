//! # jepa_t_ingest — CLI binary
//!
//! Streams a plaintext corpus file into a binary file of ternary triplets
//! for JEPA-T training on Trinity silicon.
//!
//! ## Usage
//!
//! ```text
//! jepa_t_ingest --input corpus.txt --output triplets.bin [--window-size 64] [--stride 32]
//! ```
//!
//! ## Output format
//!
//! Sequence of packed triplets, each 192 bytes:
//! - bytes   0..63  : anchor  (i8 values in {-1, 0, +1})
//! - bytes  64..127 : positive
//! - bytes 128..191 : negative
//!
//! ## License
//!
//! Apache-2.0 — Author: Dmitrii Vasilev <admin@t27.ai>

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process,
};

use clap::Parser;
use jepa_t_ingest::{ingest_text, IngestConfig};

/// JEPA-T Ternary Ingest Pipeline (Wave-14a L-S50)
///
/// Converts a plaintext corpus into binary ternary triplets for JEPA-T training.
/// Output is a raw binary stream of packed 192-byte triplet records.
#[derive(Parser, Debug)]
#[command(
    name = "jepa_t_ingest",
    version = env!("CARGO_PKG_VERSION"),
    author = "Dmitrii Vasilev <admin@t27.ai>",
    about = "Plaintext → ternary triplet pipeline for JEPA-T training on Trinity silicon"
)]
struct Args {
    /// Input plaintext corpus file (UTF-8)
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// Output binary file for ternary triplets (192 bytes each)
    #[arg(short, long, value_name = "FILE")]
    output: PathBuf,

    /// Context window size in tokens (max 64)
    #[arg(long, default_value_t = 64, value_name = "N")]
    window_size: usize,

    /// Stride between successive windows in tokens
    #[arg(long, default_value_t = 32, value_name = "N")]
    stride: usize,
}

fn main() {
    let args = Args::parse();

    let cfg = IngestConfig {
        window_size: args.window_size.min(64).max(1),
        stride: args.stride.max(1),
    };

    // Read corpus.
    let corpus = match fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "jepa_t_ingest: cannot read '{}': {}",
                args.input.display(),
                e
            );
            process::exit(1);
        }
    };

    eprintln!(
        "jepa_t_ingest: read {} bytes from '{}'",
        corpus.len(),
        args.input.display()
    );

    // Ingest into triplets.
    let triplets = ingest_text(&corpus, &cfg);

    eprintln!("jepa_t_ingest: produced {} triplets", triplets.len());

    if triplets.is_empty() {
        eprintln!("jepa_t_ingest: warning — zero triplets produced (corpus too short?)");
    }

    // Write binary output.
    let out_file = match fs::File::create(&args.output) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "jepa_t_ingest: cannot create '{}': {}",
                args.output.display(),
                e
            );
            process::exit(1);
        }
    };
    let mut writer = io::BufWriter::new(out_file);

    let mut bytes_written = 0usize;
    for triplet in &triplets {
        let bytes = triplet.to_bytes();
        match writer.write_all(&bytes) {
            Ok(()) => bytes_written += bytes.len(),
            Err(e) => {
                eprintln!("jepa_t_ingest: write error: {}", e);
                process::exit(1);
            }
        }
    }

    eprintln!(
        "jepa_t_ingest: wrote {} bytes to '{}'",
        bytes_written,
        args.output.display()
    );
    eprintln!(
        "jepa_t_ingest: done (window_size={}, stride={})",
        cfg.window_size, cfg.stride
    );
}
