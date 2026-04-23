use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match subcmd {
        "build-ext" => build_ext(),
        "build-sidepanel" => build_sidepanel(),
        "build-all" => {
            let ver = bump_build_version();
            eprintln!("[xtask] Build version: {ver}");
            build_ext();
            build_sidepanel_with_version(&ver);
        }
        "bump-version" => {
            let ver = bump_build_version();
            eprintln!("[xtask] Bumped to: {ver}");
        }
        _ => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!("  build-ext        Build trios-ext WASM (background) → dist/");
            eprintln!("  build-sidepanel  Build trios-ui-br-app WASM (sidepanel) → dist/");
            eprintln!("  build-all        Build both WASM artifacts → dist/ (auto-bumps version)");
            eprintln!("  bump-version     Increment BUILD_NUMBER in .build-version, print new version");
        }
    }
}

/// Read .build-version, increment BUILD_NUMBER, write back, return version string
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

/// Build the extension background WASM (trios-ext-ring-ex00)
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
    if !wasm_input.exists() {
        eprintln!("[xtask] ERROR: {} not found", wasm_input.display());
        std::process::exit(1);
    }

    std::fs::create_dir_all(&dist_dir).expect("create dist dir");

    eprintln!("[xtask] Running wasm-bindgen for trios-ext-ring-ex00...");
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

/// Build sidepanel WASM without version (for standalone build-sidepanel)
fn build_sidepanel() {
    build_sidepanel_with_version("dev");
}

/// Build sidepanel WASM with TRIOS_BUILD_VERSION env injected
fn build_sidepanel_with_version(ver: &str) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let bronze_dir = PathBuf::from(&manifest_dir).join("..");
    let root_workspace = PathBuf::from(&manifest_dir).join("../../../../..");
    let dist_dir = bronze_dir.join("dist");

    eprintln!("[xtask] Building trios-ui-br-app v{ver} for wasm32-unknown-unknown...");
    let status = Command::new("cargo")
        .args(["build", "-p", "trios-ui-br-app", "--target", "wasm32-unknown-unknown", "--release"])
        .env("TRIOS_BUILD_VERSION", ver)
        .current_dir(&root_workspace)
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "cargo build for trios-ui-br-app failed");

    let wasm_input = root_workspace.join("target/wasm32-unknown-unknown/release/trios_ui_br_app.wasm");
    if !wasm_input.exists() {
        eprintln!("[xtask] ERROR: {} not found", wasm_input.display());
        std::process::exit(1);
    }

    std::fs::create_dir_all(&dist_dir).expect("create dist dir");

    eprintln!("[xtask] Running wasm-bindgen for trios-ui-br-app...");
    let status = Command::new("wasm-bindgen")
        .args(["--target", "web", "--out-dir"])
        .arg(&dist_dir)
        .arg(&wasm_input)
        .status()
        .expect("failed to run wasm-bindgen");
    if !status.success() {
        eprintln!("[xtask] wasm-bindgen failed for trios-ui-br-app");
        std::process::exit(1);
    }

    report_dist(&dist_dir, "trios_ui_br_app");
    eprintln!("[xtask] Version baked in: TRIOS_BUILD_VERSION={ver}");
}

/// Print sizes of built artifacts
fn report_dist(dist_dir: &PathBuf, prefix: &str) {
    let js_out = dist_dir.join(format!("{prefix}.js"));
    let wasm_out = dist_dir.join(format!("{prefix}_bg.wasm"));
    if js_out.exists() && wasm_out.exists() {
        let js_kb = std::fs::metadata(&js_out).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0;
        let wasm_kb = std::fs::metadata(&wasm_out).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0;
        eprintln!("[xtask] SUCCESS: dist/{prefix}.js ({js_kb:.1} KB) + dist/{prefix}_bg.wasm ({wasm_kb:.1} KB)");
    } else {
        eprintln!("[xtask] WARNING: expected output files not found in dist/");
    }
}
