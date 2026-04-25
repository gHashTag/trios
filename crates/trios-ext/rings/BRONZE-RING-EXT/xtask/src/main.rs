//! xtask — build automation for BRONZE-RING-EXT
//! Reads .env from workspace root and injects API key hints as TRIOS_*_KEY_HINT env vars.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match subcmd {
        "build-ext" => build_ext(),
        "build-sidepanel" => build_sidepanel_with_version("dev", &HashMap::new()),
        "build-all" => {
            let ver = bump_build_version();
            let env_vars = load_dot_env();
            eprintln!("[xtask] Build version: {ver}");
            eprintln!("[xtask] Loaded {} vars from .env", env_vars.len());
            build_ext();
            build_sidepanel_with_version(&ver, &env_vars);
        }
        "bump-version" => {
            let ver = bump_build_version();
            eprintln!("[xtask] Bumped to: {ver}");
        }
        _ => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!("  build-all        Build both WASM artifacts, auto-bump version, inject .env");
            eprintln!("  build-ext        Build background WASM only");
            eprintln!("  build-sidepanel  Build sidepanel WASM only (dev version)");
            eprintln!("  bump-version     Increment .build-version counter");
        }
    }
}

/// Parse .env file from workspace root, return key=value map
fn load_dot_env() -> HashMap<String, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let env_path = PathBuf::from(&manifest_dir).join("../../../../..").join(".env");
    let mut map = HashMap::new();

    if !env_path.exists() {
        eprintln!("[xtask] No .env found at {}", env_path.display());
        return map;
    }

    let content = std::fs::read_to_string(&env_path).unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            // Strip inline comments (# not inside quotes)
            let val = v.split_once('#').map(|(v, _)| v).unwrap_or(v);
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(key, val);
        }
    }
    map
}

/// Read .build-version, increment, write back, return semver string
fn bump_build_version() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let version_file = PathBuf::from(&manifest_dir).join("../../../../..").join(".build-version");
    let current = std::fs::read_to_string(&version_file).unwrap_or_else(|_| "0".to_string());
    let n: u64 = current.trim().parse().unwrap_or(0);
    let next = n + 1;
    std::fs::write(&version_file, next.to_string()).expect("write .build-version");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("0.1.{next}+{ts}")
}

fn build_ext() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let bronze_dir = PathBuf::from(&manifest_dir).join("..");
    let root_workspace = PathBuf::from(&manifest_dir).join("../../../../..");
    let dist_dir = bronze_dir.join("dist");

    eprintln!("[xtask] Building trios-ext-ring-ex00 for wasm32-unknown-unknown...");
    let status = Command::new("cargo")
        .args(["build", "-p", "trios-ext-ring-ex00", "--target", "wasm32-unknown-unknown", "--release"])
        .current_dir(&root_workspace)
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "cargo build for trios-ext-ring-ex00 failed");

    let wasm_input = root_workspace.join("target/wasm32-unknown-unknown/release/trios_ext_ring_ex00.wasm");
    std::fs::create_dir_all(&dist_dir).expect("create dist dir");

    let status = Command::new("wasm-bindgen")
        .args(["--target", "no-modules", "--out-dir"])
        .arg(&dist_dir)
        .args(["--no-modules-global", "trios_ext_init"])
        .arg(&wasm_input)
        .status()
        .expect("failed to run wasm-bindgen");
    assert!(status.success(), "wasm-bindgen for trios-ext-ring-ex00 failed");
    report_dist(&dist_dir, "trios_ext_ring_ex00");
}

fn build_sidepanel_with_version(ver: &str, env_vars: &HashMap<String, String>) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let bronze_dir = PathBuf::from(&manifest_dir).join("..");
    let root_workspace = PathBuf::from(&manifest_dir).join("../../../../..");
    let dist_dir = bronze_dir.join("dist");

    // Inject key hints — mask all but last 4 chars for security
    let anthropic_hint = mask_key(env_vars.get("ANTHROPIC_API_KEY").map(|s| s.as_str()).unwrap_or(""));
    let openai_hint    = mask_key(env_vars.get("OPENAI_API_KEY").map(|s| s.as_str()).unwrap_or(""));
    let venice_hint    = mask_key(env_vars.get("VENICE_API_KEY").map(|s| s.as_str()).unwrap_or(""));
    let zai_hint       = mask_key(env_vars.get("ZAI_KEY_1").map(|s| s.as_str()).unwrap_or(""));
    let zai_api        = env_vars.get("ZAI_API").cloned().unwrap_or_default();

    if !anthropic_hint.is_empty() { eprintln!("[xtask] ANTHROPIC_API_KEY injected: {anthropic_hint}"); }
    if !openai_hint.is_empty()    { eprintln!("[xtask] OPENAI_API_KEY injected: {openai_hint}"); }
    if !venice_hint.is_empty()    { eprintln!("[xtask] VENICE_API_KEY injected: {venice_hint}"); }
    if !zai_hint.is_empty()       { eprintln!("[xtask] ZAI_KEY_1 injected: {zai_hint}"); }
    if !zai_api.is_empty()        { eprintln!("[xtask] ZAI_API: {zai_api}"); }

    eprintln!("[xtask] Building trios-ui-br-app v{ver} for wasm32-unknown-unknown...");
    let status = Command::new("cargo")
        .args(["build", "-p", "trios-ui-br-app", "--target", "wasm32-unknown-unknown", "--release"])
        .env("TRIOS_BUILD_VERSION", ver)
        .env("TRIOS_ANTHROPIC_KEY_HINT", &anthropic_hint)
        .env("TRIOS_OPENAI_KEY_HINT", &openai_hint)
        .env("TRIOS_VENICE_KEY_HINT", &venice_hint)
        .env("TRIOS_ZAI_KEY_HINT", &zai_hint)
        .env("TRIOS_ZAI_API_HINT", &zai_api)
        .current_dir(&root_workspace)
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "cargo build for trios-ui-br-app failed");

    let wasm_input = root_workspace.join("target/wasm32-unknown-unknown/release/trios_ui_br_app.wasm");
    std::fs::create_dir_all(&dist_dir).expect("create dist dir");

    let status = Command::new("wasm-bindgen")
        .args(["--target", "web", "--out-dir"])
        .arg(&dist_dir)
        .arg(&wasm_input)
        .status()
        .expect("failed to run wasm-bindgen");
    if !status.success() {
        eprintln!("[xtask] wasm-bindgen failed");
        std::process::exit(1);
    }

    report_dist(&dist_dir, "trios_ui_br_app");
    eprintln!("[xtask] Version baked in: TRIOS_BUILD_VERSION={ver}");
}

/// Mask API key — keep prefix + last 4 chars: "sk-ant-...XXXX"
fn mask_key(key: &str) -> String {
    if key.is_empty() { return String::new(); }
    if key.len() <= 8 { return "*".repeat(key.len()); }
    // Keep first 7 chars + ...last4
    let prefix = &key[..7.min(key.len())];
    let suffix = &key[key.len().saturating_sub(4)..];
    format!("{prefix}...{suffix}")
}

fn report_dist(dist_dir: &Path, prefix: &str) {
    let js_out = dist_dir.join(format!("{prefix}.js"));
    let wasm_out = dist_dir.join(format!("{prefix}_bg.wasm"));
    if js_out.exists() && wasm_out.exists() {
        let js_kb = std::fs::metadata(&js_out).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0;
        let wasm_kb = std::fs::metadata(&wasm_out).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0;
        eprintln!("[xtask] SUCCESS: dist/{prefix}.js ({js_kb:.1} KB) + dist/{prefix}_bg.wasm ({wasm_kb:.1} KB)");
    }
}
