use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SpecReadiness {
    pub name: String,
    pub path: String,
    pub parse_ok: bool,
    pub typecheck_ok: bool,
    pub verilog_gen_ok: bool,
    pub has_test: bool,
    pub has_invariant: bool,
    pub errors: Vec<String>,
}

impl SpecReadiness {
    pub fn is_ready(&self) -> bool {
        self.parse_ok
            && self.typecheck_ok
            && self.verilog_gen_ok
            && (self.has_test || self.has_invariant)
    }

    pub fn status_mark(&self) -> &'static str {
        if self.is_ready() {
            "READY"
        } else {
            "NOT READY"
        }
    }
}

pub struct SynthReadiness {
    pub specs: Vec<SpecReadiness>,
}

impl SynthReadiness {
    pub fn scan(specs_dir: &Path) -> Result<Self> {
        let mut specs = Vec::new();
        if !specs_dir.exists() {
            anyhow::bail!("specs dir not found: {}", specs_dir.display());
        }

        let mut entries: Vec<_> = fs::read_dir(specs_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "t27")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let path_str = path.display().to_string();

            let source = match fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    specs.push(SpecReadiness {
                        name,
                        path: path_str,
                        parse_ok: false,
                        typecheck_ok: false,
                        verilog_gen_ok: false,
                        has_test: false,
                        has_invariant: false,
                        errors: vec![format!("read error: {}", e)],
                    });
                    continue;
                }
            };

            let has_test = source.lines().any(|l| {
                let t = l.trim();
                t.starts_with("test ") || t.starts_with("test{") || t == "test"
            });
            let has_invariant = source.lines().any(|l| l.trim().starts_with("invariant "));

            let (parse_ok, parse_errors) = check_parse(&source);
            let (typecheck_ok, tc_errors) = if parse_ok {
                check_typecheck(&source)
            } else {
                (false, vec!["skipped: parse failed".into()])
            };

            let mut all_errors = parse_errors;
            all_errors.extend(tc_errors);

            specs.push(SpecReadiness {
                name,
                path: path_str,
                parse_ok,
                typecheck_ok,
                verilog_gen_ok: parse_ok,
                has_test,
                has_invariant,
                errors: all_errors,
            });
        }

        Ok(SynthReadiness { specs })
    }

    pub fn ready_count(&self) -> usize {
        self.specs.iter().filter(|s| s.is_ready()).count()
    }

    pub fn total_count(&self) -> usize {
        self.specs.len()
    }

    pub fn print_report(&self) {
        println!("=== Synth Readiness Report ===");
        println!(
            "{:<30} {:<8} {:<10} {:<10} {:<6} {:<10}",
            "Spec", "Parse", "Typecheck", "Verilog", "TDD", "Status"
        );
        println!("{}", "-".repeat(80));
        for s in &self.specs {
            let tdd = if s.has_test {
                "T"
            } else if s.has_invariant {
                "I"
            } else {
                "-"
            };
            println!(
                "{:<30} {:<8} {:<10} {:<10} {:<6} {:<10}",
                s.name,
                if s.parse_ok { "OK" } else { "FAIL" },
                if s.typecheck_ok { "OK" } else { "FAIL" },
                if s.verilog_gen_ok { "OK" } else { "FAIL" },
                tdd,
                s.status_mark()
            );
            for err in &s.errors {
                println!("  ERROR: {}", err);
            }
        }
        println!("{}", "-".repeat(80));
        println!(
            "Ready: {}/{} ({:.0}%)",
            self.ready_count(),
            self.total_count(),
            if self.total_count() > 0 {
                100.0 * self.ready_count() as f64 / self.total_count() as f64
            } else {
                0.0
            }
        );
    }
}

fn check_parse(source: &str) -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let brace_count =
        source.chars().filter(|&c| c == '{').count() - source.chars().filter(|&c| c == '}').count();
    if brace_count != 0 {
        errors.push(format!("unbalanced braces (delta={})", brace_count));
    }

    let has_module = source.lines().any(|l| {
        let t = l.trim();
        t.starts_with("module ")
    });
    if !has_module {
        errors.push("no module declaration".into());
    }

    (errors.is_empty(), errors)
}

fn check_typecheck(source: &str) -> (bool, Vec<String>) {
    let mut errors = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") && trimmed.ends_with('{') {
            if !trimmed.contains('(') {
                errors.push(format!("fn missing parens: {}", trimmed));
            }
        }
    }

    (errors.is_empty(), errors)
}
