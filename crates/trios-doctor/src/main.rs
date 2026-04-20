use trios_doctor::{CheckStatus, Doctor};

fn main() -> anyhow::Result<()> {
    let workspace = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let root = std::path::Path::new(&workspace)
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    eprintln!("=== trios-doctor: workspace diagnostics ===\n");
    eprintln!("Workspace: {}", root.display());

    let doctor = Doctor::new(&root);
    let crate_count = doctor.count_crates();
    eprintln!("Crates:    {}\n", crate_count);

    let diag = doctor.run_all();

    let mut any_red = false;
    for check in &diag.checks {
        let (icon, label) = match check.status {
            CheckStatus::Green => ("OK", "GREEN"),
            CheckStatus::Yellow => ("WARN", "YELLOW"),
            CheckStatus::Red => {
                any_red = true;
                ("FAIL", "RED")
            }
        };
        eprintln!("[{}] {} {}", icon, label, check.name);
        if check.status != CheckStatus::Green {
            for line in check.message.lines().take(5) {
                eprintln!("    {}", line);
            }
            if !check.failed_crates.is_empty() {
                eprintln!("    Affected: {}", check.failed_crates.join(", "));
            }
        } else {
            eprintln!("    {}", check.message);
        }
        eprintln!();
    }

    eprintln!("=== {} crates diagnosed ===", crate_count);

    if any_red {
        std::process::exit(1);
    }

    Ok(())
}
