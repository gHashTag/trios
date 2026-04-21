use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("build-ext") => build_ext(),
        Some("ci-check") => ci_check(),
        _ => {
            eprintln!("Usage: cargo xtask <build-ext|ci-check>");
            std::process::exit(1);
        }
    }
}

fn root() -> std::path::PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|p| {
            Path::new(&p)
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf()
        })
        .unwrap_or_else(|_| ".".into())
}

fn build_ext() {
    let r = root();
    let ext = r.join("extension");
    let dist = ext.join("dist");

    let st = Command::new("wasm-pack")
        .args([
            "build",
            "crates/trios-ext",
            "--target",
            "web",
            "--out-dir",
            "../../extension/dist/wasm",
        ])
        .current_dir(&r)
        .status()
        .expect("wasm-pack not found");
    assert!(st.success());

    fs::write(dist.join("bootstrap.js"),
        "import init, { run } from \"./wasm/trios_ext.js\";\nawait init(\"./wasm/trios_ext_bg.wasm\");\nrun();\n").unwrap();
    fs::write(dist.join("bg-sw.js"),
        "import init, { init_background } from \"./wasm/trios_ext.js\";\nawait init(\"./wasm/trios_ext_bg.wasm\");\ninit_background();\n").unwrap();

    for f in &[
        "bg-sw.js",
        "trios_ext_bg.wasm",
        "trios_ext.js",
        "sidepanel.js",
        "style.css",
    ] {
        let _ = fs::remove_file(ext.join(f));
    }
    assert!(find_js_outside_dist(&ext).is_empty(), "L9 VIOLATION");
    eprintln!("[OK] Zero JS outside dist/");
}

fn ci_check() {
    let ext = root().join("extension");

    let js = find_js_outside_dist(&ext);
    if !js.is_empty() {
        eprintln!("L9 VIOLATION:");
        for f in &js {
            eprintln!("  {}", f.display());
        }
        std::process::exit(1);
    }
    eprintln!("[L9] OK");

    let html = fs::read_to_string(ext.join("sidepanel.html")).unwrap();
    if html
        .lines()
        .any(|l| l.contains("<script") && !l.contains("src="))
    {
        eprintln!("CSP VIOLATION: inline <script>");
        std::process::exit(1);
    }
    eprintln!("[CSP] OK");

    for f in &[
        "manifest.json",
        "sidepanel.html",
        "dist/bootstrap.js",
        "dist/bg-sw.js",
        "dist/wasm/trios_ext.js",
        "dist/wasm/trios_ext_bg.wasm",
        "icons/icon-16.png",
    ] {
        assert!(ext.join(f).exists(), "MISSING: {}", f);
    }
    eprintln!("[OK] All checks passed");
}

fn find_js_outside_dist(ext: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(ext)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "js").unwrap_or(false))
        .map(|e| e.path())
        .collect()
}
