//! `ingest_appendices` — SSOT-first ingestion of `docs/phd/appendix/*.tex` into
//! `ssot.chapters` (kind='appendix').
//!
//! Anchor: phi^2 + phi^-2 = 3.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).
//!
//! R1 (CROWN): Rust only. No Python, no shell.
//! R5 (HONEST): if a row already exists for an appendix slug we UPDATE only
//!   `body_md`, `title`, and `updated_at` — never silently fabricate citations.
//! R6 (SSOT): writes to Railway PostgreSQL `ssot.chapters`; the local .tex
//!   files become *derived* artefacts of the row that survives this command.
//!
//! Usage:
//!     export DATABASE_URL="$RAILWAY_SSOT_URL"
//!     cargo run -p trios-phd --features rag --release --bin ingest_appendices -- \
//!         --appendix-dir docs/phd/appendix
//!
//! The binary is idempotent.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio_postgres::NoTls;

#[derive(Parser, Debug)]
#[command(
    author = "Dmitrii Vasilev <ORCID 0009-0008-4294-6159>",
    about = "Ingest docs/phd/appendix/*.tex into ssot.chapters (kind='appendix')."
)]
struct Cli {
    #[arg(long)]
    database_url: Option<String>,
    #[arg(long, default_value = "docs/phd/appendix")]
    appendix_dir: PathBuf,
    /// Print plan without writing.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

/// One mapping from on-disk filename stem to canonical SSOT slug + chapter_no
/// + display title. Chapter numbers reserve 1000+ to avoid collisions with
/// FA chapters (1..34) and any future Trinity ch_NN ingestion (100..134).
struct AppendixSpec {
    file_stem: &'static str,
    slug: &'static str,
    chapter_no: i32,
    title: &'static str,
}

const APPENDICES: &[AppendixSpec] = &[
    AppendixSpec {
        file_stem: "A-catalogue",
        slug: "ap-a-catalogue",
        chapter_no: 1001,
        title: "Appendix A — Catalogue of Theorems & Definitions",
    },
    AppendixSpec {
        file_stem: "B-falsification",
        slug: "ap-b-falsification",
        chapter_no: 1002,
        title: "Appendix B — Falsifiability of phi-Numeric Claims",
    },
    AppendixSpec {
        file_stem: "C-golden-benchmark",
        slug: "ap-c-golden-benchmark",
        chapter_no: 1003,
        title: "Appendix C — Golden Benchmark: GF4/GF8/GF16 Tables",
    },
    AppendixSpec {
        file_stem: "D-golden-mirror",
        slug: "ap-d-golden-mirror",
        chapter_no: 1004,
        title: "Appendix D — Golden Mirror: Trinity ↔ Flos Aureus Symmetry",
    },
    AppendixSpec {
        file_stem: "E-lexicon",
        slug: "ap-e-lexicon",
        chapter_no: 1005,
        title: "Appendix E — Lexicon of phi-Numeric Terms",
    },
    AppendixSpec {
        file_stem: "F-coq-citation-map",
        slug: "ap-f-coq-citation-map",
        chapter_no: 1006,
        title: "Appendix F — Coq Citation Map (R14)",
    },
    AppendixSpec {
        file_stem: "F-fpga-bitstream",
        slug: "ap-f-fpga-bitstream",
        chapter_no: 1007,
        title: "Appendix F — FPGA Bitstream Archive (iCE40 + SHA-256)",
    },
    AppendixSpec {
        file_stem: "G-data-availability",
        slug: "ap-g-data-availability",
        chapter_no: 1008,
        title: "Appendix G — Data Availability (ACM AE)",
    },
    AppendixSpec {
        file_stem: "H-acm-ae-checklist",
        slug: "ap-h-acm-ae-checklist",
        chapter_no: 1009,
        title: "Appendix H — ACM Artefact Evaluation Checklist",
    },
    AppendixSpec {
        file_stem: "H-zenodo-doi",
        slug: "ap-h-zenodo-doi",
        chapter_no: 1010,
        title: "Appendix H — Zenodo DOI Registry (13 records)",
    },
    AppendixSpec {
        file_stem: "I-xdc-pin-map",
        slug: "ap-i-xdc-pin-map",
        chapter_no: 1011,
        title: "Appendix I — XDC Pin Map: QMTech XC7A100T/200T",
    },
    AppendixSpec {
        file_stem: "J-troubleshooting",
        slug: "ap-j-troubleshooting",
        chapter_no: 1012,
        title: "Appendix J — Hardware Troubleshooting Log (BLK-001..005)",
    },
    AppendixSpec {
        file_stem: "K-agent-memory",
        slug: "ap-k-agent-memory",
        chapter_no: 1013,
        title: "Appendix K — Agent Memory & Replay Protocol",
    },
    AppendixSpec {
        file_stem: "L-pollen-channel",
        slug: "ap-l-pollen-channel",
        chapter_no: 1014,
        title: "Appendix L — Pollen Channel: Inter-Agent Hand-off",
    },
];

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let database_url = cli
        .database_url
        .clone()
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow!("--database-url or DATABASE_URL is required"))?;

    eprintln!("[ingest_appendices] connecting to railway postgres ...");
    let (db, conn) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .with_context(|| "connect to DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[ingest_appendices] postgres connection error: {e}");
        }
    });

    let mut written = 0usize;
    let mut skipped_missing = 0usize;
    let mut total_chars: usize = 0;

    for spec in APPENDICES {
        let path = cli.appendix_dir.join(format!("{}.tex", spec.file_stem));
        if !path.exists() {
            eprintln!("[ingest_appendices] SKIP missing file: {}", path.display());
            skipped_missing += 1;
            continue;
        }
        let body = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        // The body_md column accepts any text; we store the raw .tex as the
        // canonical body so the SSOT row is byte-faithful to the artefact
        // included by main.tex. Markdown-mirror and pandoc-rendered prose
        // can be regenerated downstream from body_md if/when needed.
        total_chars += body.len();

        if cli.dry_run {
            eprintln!(
                "[ingest_appendices] DRY {} <- {} ({} chars)",
                spec.slug,
                path.display(),
                body.len()
            );
            continue;
        }

        // UPSERT: insert if missing, update body/title/timestamps otherwise.
        // citations and coq_refs default to '[]' on insert and are left
        // untouched on update (R5: never fabricate provenance).
        db.execute(
            "INSERT INTO ssot.chapters \
                 (chapter_slug, chapter_no, kind, title, body_md, citations, coq_refs, updated_at) \
             VALUES ($1, $2, 'appendix', $3, $4, '[]'::jsonb, '[]'::jsonb, now()) \
             ON CONFLICT (chapter_slug) DO UPDATE SET \
                 chapter_no = EXCLUDED.chapter_no, \
                 kind       = 'appendix', \
                 title      = EXCLUDED.title, \
                 body_md    = EXCLUDED.body_md, \
                 updated_at = now()",
            &[&spec.slug, &spec.chapter_no, &spec.title, &body],
        )
        .await
        .with_context(|| format!("upsert {}", spec.slug))?;
        eprintln!(
            "[ingest_appendices] OK  {} <- {} ({} chars)",
            spec.slug,
            path.display(),
            body.len()
        );
        written += 1;
    }

    eprintln!(
        "[ingest_appendices] done: written={} skipped_missing={} total_chars={}",
        written, skipped_missing, total_chars
    );
    Ok(())
}
