use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match subcmd {
        "build-ext" => build_ext(),
        "build-sidepanel" => build_sidepanel(),
        "build-all" => {
            build_ext();
            build_sidepanel();
        }
        _ => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!("  build-ext        Build trios-ext WASM (background) → dist/");
            eprintln!("  build-sidepanel  Build trios-ui-br-app WASM (sidepanel) → dist/");
            eprintln!("  build-all        Build both WASM artifacts → dist/");
        }
    }
}

/// Build the extension background WASM (trios-ext-ring-ex00)
fn build_ext() {
    // Both trios-ext-ring-ex00 and trios-ui-br-app are root workspace members
    // xtask/ → BRONZE-RING-EXT/ → rings/ → trios-ext/ → crates/ → root workspace
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let bronze_dir = PathBuf::from(&manifest_dir).join("..");
    let root_workspace = PathBuf::from(&manifest_dir).join("../../../../..");

    let dist_dir = bronze_dir.join("dist");

    eprintln!("[xtask] Building trios-ext-ring-ex00 for wasm32-unknown-unknown...");
    let status = Command::new("cargo")
        .args([
            "build",
            "-p",
            "trios-ext-ring-ex00",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
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
        .arg("--target")
        .arg("no-modules")
        .arg("--out-dir")
        .arg(&dist_dir)
        .arg("--no-modules-global")
        .arg("trios_ext_init")
        .arg(&wasm_input)
        .status()
        .expect("failed to run wasm-bindgen");
    assert!(status.success(), "wasm-bindgen for trios-ext-ring-ex00 failed");

    let js_out = dist_dir.join("trios_ext_ring_ex00.js");
    let wasm_out = dist_dir.join("trios_ext_ring_ex00_bg.wasm");
    if js_out.exists() && wasm_out.exists() {
        let js_size = std::fs::metadata(&js_out).map(|m| m.len()).unwrap_or(0);
        let wasm_size = std::fs::metadata(&wasm_out).map(|m| m.len()).unwrap_or(0);
        eprintln!(
            "[xtask] SUCCESS: dist/trios_ext_ring_ex00.js ({:.1} KB) + dist/trios_ext_ring_ex00_bg.wasm ({:.1} KB)",
            js_size as f64 / 1024.0,
            wasm_size as f64 / 1024.0
        );
    } else {
        eprintln!("[xtask] WARNING: expected output files not found");
    }
}

/// Build the sidepanel UI WASM (trios-ui-br-app — Dioxus sidebar)
fn build_sidepanel() {
    // xtask/ → BRONZE-RING-EXT/ → rings/ → trios-ext/ → crates/ → root workspace
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let bronze_dir = PathBuf::from(&manifest_dir).join("..");
    let root_workspace = PathBuf::from(&manifest_dir).join("../../../../..");

    let dist_dir = bronze_dir.join("dist");

    eprintln!("[xtask] Building trios-ui-br-app for wasm32-unknown-unknown...");
    let status = Command::new("cargo")
        .args([
            "build",
            "-p",
            "trios-ui-br-app",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ])
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
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&dist_dir)
        .arg(&wasm_input)
        .status()
        .expect("failed to run wasm-bindgen");

    if !status.success() {
        eprintln!("[xtask] wasm-bindgen failed for trios-ui-br-app");
        std::process::exit(1);
    }

    let js_out = dist_dir.join("trios_ui_br_app.js");
    let wasm_out = dist_dir.join("trios_ui_br_app_bg.wasm");
    if js_out.exists() && wasm_out.exists() {
        let js_size = std::fs::metadata(&js_out).map(|m| m.len()).unwrap_or(0);
        let wasm_size = std::fs::metadata(&wasm_out).map(|m| m.len()).unwrap_or(0);
        eprintln!(
            "[xtask] SUCCESS: dist/trios_ui_br_app.js ({:.1} KB) + dist/trios_ui_br_app_bg.wasm ({:.1} KB)",
            js_size as f64 / 1024.0,
            wasm_size as f64 / 1024.0
        );
    } else {
        eprintln!("[xtask] WARNING: expected output files not found");
    }
}
