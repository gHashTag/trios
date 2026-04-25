use std::{env, fs, path::PathBuf};

fn main() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pkg_dir = crate_dir.join("pkg");
    let dist_dir = crate_dir.join("../../extension").join("dist");
    fs::create_dir_all(&dist_dir).expect("failed to create extension/dist");
    let artifacts = ["trios_ext_br.js", "trios_ext_br_bg.wasm", "trios_ext_br_bg.wasm.d.ts"];
    let mut copied = 0;
    for name in &artifacts {
        let src = pkg_dir.join(name);
        let dst = dist_dir.join(name);
        if src.exists() {
            fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {}: {}", name, e));
            copied += 1;
        }
    }
    // Also copy with legacy names for backward compatibility
    let legacy_map = [
        ("trios_ext_br.js", "trios_ext.js"),
        ("trios_ext_br_bg.wasm", "trios_ext_bg.wasm"),
    ];
    for (src_name, dst_name) in &legacy_map {
        let src = pkg_dir.join(src_name);
        let dst = dist_dir.join(dst_name);
        if src.exists() {
            let _ = fs::copy(&src, &dst);
        }
    }
    if copied > 0 {
        println!("cargo:warning=build.rs: copied {} artifacts pkg/ -> extension/dist/", copied);
    }
    println!("cargo:rerun-if-changed=pkg/");
}
