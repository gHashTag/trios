use trios_doctor::Doctor;

fn main() -> anyhow::Result<()> {
    let workspace = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let root = std::path::Path::new(&workspace)
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    println!("=== trios-doctor: workspace diagnostics ===\n");
    println!("Workspace: {}\n", root.display());

    let doctor = Doctor::new(&root);
    let diagnoses = doctor.run_all();

    let mut green = 0;
    let mut yellow = 0;
    let mut red = 0;

    for d in &diagnoses {
        let mut crate_status = "GREEN";
        for c in &d.checks {
            match c.status {
                trios_doctor::CheckStatus::Red => {
                    crate_status = "RED";
                    red += 1;
                }
                trios_doctor::CheckStatus::Yellow => {
                    if crate_status != "RED" {
                        crate_status = "YELLOW";
                    }
                    yellow += 1;
                }
                trios_doctor::CheckStatus::Green => {
                    green += 1;
                }
            }
        }

        let icon = match crate_status {
            "GREEN" => "✅",
            "YELLOW" => "⚠️",
            _ => "❌",
        };
        println!("{} {} [{}]", icon, d.crate_name, crate_status);

        for c in &d.checks {
            if c.status != trios_doctor::CheckStatus::Green {
                println!(
                    "    {} {}: {}",
                    match c.status {
                        trios_doctor::CheckStatus::Red => "🔴",
                        trios_doctor::CheckStatus::Yellow => "🟡",
                        trios_doctor::CheckStatus::Green => "🟢",
                    },
                    c.name,
                    c.message.lines().next().unwrap_or("")
                );
            }
        }
    }

    println!(
        "\n=== Summary: {} checks: {} green, {} yellow, {} red ===",
        green + yellow + red,
        green,
        yellow,
        red
    );

    if red > 0 {
        std::process::exit(1);
    }

    Ok(())
}
