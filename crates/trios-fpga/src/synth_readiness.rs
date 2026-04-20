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

#[derive(Debug)]
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
    let open_count = source.chars().filter(|&c| c == '{').count();
    let close_count = source.chars().filter(|&c| c == '}').count();
    if open_count != close_count {
        let delta = open_count as i32 - close_count as i32;
        errors.push(format!("unbalanced braces (delta={})", delta));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_spec(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{}.t27", name));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn spec_ready_when_all_pass_with_test() {
        let s = SpecReadiness {
            name: "good".into(),
            path: "good.t27".into(),
            parse_ok: true,
            typecheck_ok: true,
            verilog_gen_ok: true,
            has_test: true,
            has_invariant: false,
            errors: vec![],
        };
        assert!(s.is_ready());
        assert_eq!(s.status_mark(), "READY");
    }

    #[test]
    fn spec_ready_when_all_pass_with_invariant() {
        let s = SpecReadiness {
            name: "good".into(),
            path: "good.t27".into(),
            parse_ok: true,
            typecheck_ok: true,
            verilog_gen_ok: true,
            has_test: false,
            has_invariant: true,
            errors: vec![],
        };
        assert!(s.is_ready());
    }

    #[test]
    fn spec_not_ready_when_parse_fails() {
        let s = SpecReadiness {
            name: "bad".into(),
            path: "bad.t27".into(),
            parse_ok: false,
            typecheck_ok: true,
            verilog_gen_ok: true,
            has_test: true,
            has_invariant: false,
            errors: vec!["parse error".into()],
        };
        assert!(!s.is_ready());
        assert_eq!(s.status_mark(), "NOT READY");
    }

    #[test]
    fn spec_not_ready_when_no_tdd() {
        let s = SpecReadiness {
            name: "no_tdd".into(),
            path: "no_tdd.t27".into(),
            parse_ok: true,
            typecheck_ok: true,
            verilog_gen_ok: true,
            has_test: false,
            has_invariant: false,
            errors: vec![],
        };
        assert!(!s.is_ready());
    }

    #[test]
    fn spec_not_ready_when_typecheck_fails() {
        let s = SpecReadiness {
            name: "tc_fail".into(),
            path: "tc_fail.t27".into(),
            parse_ok: true,
            typecheck_ok: false,
            verilog_gen_ok: true,
            has_test: true,
            has_invariant: false,
            errors: vec!["typecheck error".into()],
        };
        assert!(!s.is_ready());
    }

    #[test]
    fn scan_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.total_count(), 0);
        assert_eq!(readiness.ready_count(), 0);
    }

    #[test]
    fn scan_nonexistent_dir_fails() {
        let result = SynthReadiness::scan(std::path::Path::new("/nonexistent/path"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn scan_valid_spec_with_test() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(
            dir.path(),
            "good",
            "module Good;\nfn foo() {}\ntest good_test {}\n",
        );
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.total_count(), 1);
        assert_eq!(readiness.ready_count(), 1);
        assert!(readiness.specs[0].parse_ok);
        assert!(readiness.specs[0].has_test);
    }

    #[test]
    fn scan_valid_spec_with_invariant() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(
            dir.path(),
            "inv",
            "module Inv;\nfn bar() {}\ninvariant bar_pos {}\n",
        );
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.ready_count(), 1);
        assert!(readiness.specs[0].has_invariant);
    }

    #[test]
    fn scan_spec_no_module_fails_parse() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(dir.path(), "no_mod", "fn foo() {}\ntest t {}\n");
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.ready_count(), 0);
        assert!(!readiness.specs[0].parse_ok);
        assert!(readiness.specs[0]
            .errors
            .iter()
            .any(|e| e.contains("no module")));
    }

    #[test]
    fn scan_spec_unbalanced_braces_fails() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(
            dir.path(),
            "unbal",
            "module Unbal;\nfn foo() {\ntest t {}\n",
        );
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert!(!readiness.specs[0].parse_ok);
        assert!(readiness.specs[0]
            .errors
            .iter()
            .any(|e| e.contains("unbalanced")));
    }

    #[test]
    fn scan_spec_no_tdd_not_ready() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(dir.path(), "no_tdd", "module NoTdd;\nfn foo() {}\n");
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.total_count(), 1);
        assert_eq!(readiness.ready_count(), 0);
        assert!(!readiness.specs[0].has_test);
        assert!(!readiness.specs[0].has_invariant);
    }

    #[test]
    fn scan_ignores_non_t27_files() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(dir.path(), "good", "module G;\nfn f() {}\ntest t {}\n");
        let txt = dir.path().join("readme.txt");
        fs::write(&txt, "not a spec").unwrap();
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.total_count(), 1);
    }

    #[test]
    fn scan_multiple_specs_mixed() {
        let dir = tempfile::tempdir().unwrap();
        write_spec(dir.path(), "a_good", "module A;\nfn f() {}\ntest t {}\n");
        write_spec(dir.path(), "b_bad", "no module here\ntest t {}\n");
        write_spec(dir.path(), "c_no_tdd", "module C;\nfn f() {}\n");
        let readiness = SynthReadiness::scan(dir.path()).unwrap();
        assert_eq!(readiness.total_count(), 3);
        assert_eq!(readiness.ready_count(), 1);
    }

    #[test]
    fn check_parse_balanced_with_module() {
        let (ok, errs) = check_parse("module Foo;\nfn bar() {}\n");
        assert!(ok);
        assert!(errs.is_empty());
    }

    #[test]
    fn check_parse_no_module() {
        let (ok, errs) = check_parse("fn bar() {}");
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("no module")));
    }

    #[test]
    fn check_parse_unbalanced_open() {
        let (ok, errs) = check_parse("module Foo;\nfn bar() {");
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("unbalanced")));
    }

    #[test]
    fn check_parse_unbalanced_close() {
        let (ok, _errs) = check_parse("module Foo;\n}}");
        assert!(!ok);
    }

    #[test]
    fn check_typecheck_valid_fn() {
        let (ok, errs) = check_typecheck("fn foo(x: i32) {");
        assert!(ok);
        assert!(errs.is_empty());
    }

    #[test]
    fn check_typecheck_fn_no_parens() {
        let (ok, errs) = check_typecheck("fn badfn {");
        assert!(!ok);
        assert!(errs.iter().any(|e| e.contains("missing parens")));
    }

    #[test]
    fn check_typecheck_fn_with_body_no_brace() {
        let (ok, _) = check_typecheck("fn foo(x: i32)");
        assert!(ok);
    }

    #[test]
    fn print_report_no_panic() {
        let readiness = SynthReadiness {
            specs: vec![SpecReadiness {
                name: "test_spec".into(),
                path: "/tmp/test.t27".into(),
                parse_ok: true,
                typecheck_ok: true,
                verilog_gen_ok: true,
                has_test: true,
                has_invariant: false,
                errors: vec![],
            }],
        };
        readiness.print_report();
    }
}
