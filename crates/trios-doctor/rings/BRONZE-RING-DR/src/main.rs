//! BRONZE-RING-DR — trios-doctor CLI entry point
//! Orchestrates: DR-01 (check) → DR-02 (heal) → DR-03 (report)
//! NO business logic here — all logic in Silver rings.

use clap::{Parser, Subcommand};
use trios_doctor_dr00::CheckStatus;
use trios_doctor_dr01::Doctor;
use trios_doctor_dr02::Healer;
use trios_doctor_dr03::Reporter;

#[derive(Parser)]
#[command(name = "trios-doctor")]
#[command(about = "Leukocyte agent: autonomous diagnostics, healing, and quick-win repair")]
#[command(version)]
struct Cli {
    /// Workspace root directory (default: current directory)
    #[arg(long, global = true, default_value = ".")]
    workspace: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run diagnostics checks on workspace
    Check {
        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Output results in SARIF format (for GitHub Code Scanning)
        #[arg(long)]
        sarif: bool,

        /// Output results in GitHub Actions format (::error / ::warning)
        #[arg(long)]
        github: bool,
    },

    /// Auto-fix Yellow-level issues (dry-run by default)
    Heal {
        /// Actually apply fixes (default: dry-run only)
        #[arg(long, default_value_t = true)]
        dry_run: bool,

        /// Run checks after healing to verify
        #[arg(long)]
        verify: bool,
    },

    /// Generate full diagnostic report
    Report {
        /// Output report as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let workspace = if cli.workspace == "." {
        std::env::current_dir()?.to_string_lossy().to_string()
    } else {
        cli.workspace.clone()
    };

    match cli.command {
        Commands::Check {
            json,
            sarif,
            github,
        } => {
            let doctor = Doctor::new(&workspace);
            let diagnosis = doctor.run_all();

            if json {
                Reporter::print_json(&diagnosis);
            } else if sarif {
                println!("{}", Reporter::report_sarif(&diagnosis));
            } else if github {
                println!("{}", Reporter::report_github(&diagnosis));
            } else {
                Reporter::print_text(&diagnosis);
            }

            if Reporter::overall_status(&diagnosis) == CheckStatus::Red {
                std::process::exit(1);
            }
        }

        Commands::Heal { dry_run, verify } => {
            let doctor = Doctor::new(&workspace);
            let diagnosis = doctor.run_all();

            eprintln!("=== trios-doctor heal ===\n");
            eprintln!("Mode: {}\n", if dry_run { "DRY-RUN" } else { "LIVE" });

            let healer = Healer::new(&workspace).with_dry_run(dry_run);
            let result = healer.heal(&diagnosis);

            if !result.fixed.is_empty() {
                eprintln!("Fixed:");
                for item in &result.fixed {
                    eprintln!("  ✅ {}", item);
                }
            }
            if !result.skipped.is_empty() {
                eprintln!("\nSkipped (manual):");
                for item in &result.skipped {
                    eprintln!("  ⏭️  {}", item);
                }
            }
            if !result.failed.is_empty() {
                eprintln!("\nFailed:");
                for item in &result.failed {
                    eprintln!("  ❌ {}", item);
                }
            }

            if verify {
                eprintln!("\n--- Verification ---");
                let diagnosis2 = doctor.run_all();
                Reporter::print_text(&diagnosis2);

                if Reporter::overall_status(&diagnosis2) == CheckStatus::Red {
                    std::process::exit(1);
                }
            }
        }

        Commands::Report { json } => {
            let doctor = Doctor::new(&workspace);
            let diagnosis = doctor.run_all();

            if json {
                println!("{}", Reporter::report_json(&diagnosis));
            } else {
                println!("{}", Reporter::report_human(&diagnosis));
            }

            eprintln!("{}", Reporter::summary_line(&diagnosis));
        }
    }

    Ok(())
}
