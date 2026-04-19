use std::path::PathBuf;
use std::process::Command;

fn main() {
    let zig_path = "vendor/zig-crypto-mining";

    if PathBuf::from(zig_path).join("build.zig").exists() {
        let _ = Command::new("zig")
            .args(["build", "-Doptimize=ReleaseFast"])
            .current_dir(zig_path)
            .status();
    }

    // Always link if the static lib exists (may have been built manually or in a previous run)
    let lib_dir = std::env::current_dir()
        .unwrap()
        .join(zig_path)
        .join("zig-out/lib");
    let static_lib = lib_dir.join("libcrypto_mining.a");
    if static_lib.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=crypto_mining");
    }

    println!("cargo:rerun-if-changed={}/src", zig_path);
}
