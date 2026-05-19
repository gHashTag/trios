//! trinity-extract — Coq proof extractor for IGLA assertions
//! Reads trinity-clara/proofs/igla/*.v, emits assertions/igla_assertions.json
//!
//! L-R1 compliant: RUST ONLY — replaces any Python extract script
//! Usage: cargo run -p trinity-extract -- --input trinity-clara/proofs/igla --output assertions/igla_assertions.json

use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, serde::Serialize)]
struct ExtractedConstant {
    name: String,
    value: String,
    source_file: String,
    source_line: usize,
    status: ProofStatus,
}

#[derive(Debug, serde::Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum ProofStatus {
    Proven,
    Admitted,
}

#[derive(Debug, serde::Serialize)]
struct ExtractionResult {
    schema_version: String,
    source_commit: String,
    generated_by: String,
    theorem_count: usize,
    admitted_count: usize,
    proven_count: usize,
    constants: Vec<ExtractedConstant>,
}

fn get_git_commit(repo_path: &Path) -> String {
    std::process::Command::new("git")
        .args(["-C", repo_path.to_str().unwrap_or("."), "rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn detect_status(content: &str, theorem_name: &str) -> ProofStatus {
    // Find the theorem block and check if it ends with Admitted or Qed
    let mut in_theorem = false;
    let mut _depth = 0usize;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains(theorem_name) && (trimmed.starts_with("Theorem") || trimmed.starts_with("Lemma")) {
            in_theorem = true;
        }
        if in_theorem {
            if trimmed.contains("Proof.") { _depth += 1; }
            if trimmed == "Admitted." {
                return ProofStatus::Admitted;
            }
            if trimmed == "Qed." {
                return ProofStatus::Proven;
            }
        }
    }
    ProofStatus::Admitted // conservative default
}

fn extract_definitions(content: &str, source_file: &str) -> Vec<ExtractedConstant> {
    let mut results = Vec::new();
    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Match: Definition <name> := <value>.
        if let Some(rest) = trimmed.strip_prefix("Definition ") {
            if let Some(assign_pos) = rest.find(" := ") {
                let name = rest[..assign_pos].trim().to_string();
                let raw_value = rest[assign_pos + 4..].trim();
                let value = raw_value.trim_end_matches('.').to_string();
                // Only extract numeric or phi-related definitions
                let is_numeric = value.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
                let is_phi = value.contains("phi") || value.contains("φ") || value.contains("sqrt");
                if is_numeric || is_phi {
                    results.push(ExtractedConstant {
                        name,
                        value,
                        source_file: source_file.to_string(),
                        source_line: line_idx + 1,
                        status: ProofStatus::Proven, // definitions are always valid
                    });
                }
            }
        }
        // Match: Theorem/Lemma <name>
        if trimmed.starts_with("Theorem ") || trimmed.starts_with("Lemma ") {
            let rest = trimmed
                .trim_start_matches("Theorem ")
                .trim_start_matches("Lemma ");
            let name = rest.split_whitespace().next().unwrap_or("").trim_end_matches(':').to_string();
            if !name.is_empty() {
                let status = detect_status(content, &name);
                results.push(ExtractedConstant {
                    name: format!("theorem::{name}"),
                    value: "(see .v file)".to_string(),
                    source_file: source_file.to_string(),
                    source_line: line_idx + 1,
                    status,
                });
            }
        }
    }
    results
}

fn process_directory(dir: &Path) -> Vec<ExtractedConstant> {
    let mut all = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return all; };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("v"))
        .collect();
    paths.sort();
    for path in paths {
        let Ok(content) = fs::read_to_string(&path) else { continue; };
        let source = path.to_string_lossy().to_string();
        all.extend(extract_definitions(&content, &source));
    }
    all
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input_dir = args.iter()
        .position(|a| a == "--input")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("trinity-clara/proofs/igla"));
    let output_file = args.iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assertions/igla_assertions.json"));

    let constants = process_directory(&input_dir);
    let admitted_count = constants.iter().filter(|c| c.status == ProofStatus::Admitted).count();
    let proven_count = constants.iter().filter(|c| c.status == ProofStatus::Proven).count();
    let theorem_count = constants.iter().filter(|c| c.name.starts_with("theorem::")).count();

    let commit = get_git_commit(Path::new("."));
    let result = ExtractionResult {
        schema_version: "1.0.0".to_string(),
        source_commit: commit,
        generated_by: "crates/trinity-extract/src/main.rs (L-R1: RUST ONLY)".to_string(),
        theorem_count,
        admitted_count,
        proven_count,
        constants,
    };

    let json = serde_json::to_string_pretty(&result).expect("JSON serialization failed");
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&output_file, &json).expect("Failed to write assertions JSON");
    println!("✅ trinity-extract: {theorem_count} theorems ({proven_count} Proven, {admitted_count} Admitted)");
    println!("   output: {}", output_file.display());
    println!("   φ²+φ⁻²=3 | TRINITY | L-R14");
}
