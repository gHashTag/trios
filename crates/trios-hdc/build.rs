use std::path::PathBuf;
use std::process::Command;

fn main() {
    let zig_path = "vendor/zig-hdc";

    if PathBuf::from(zig_path).join("build.zig").exists() {
        let _ = Command::new("zig")
            .args(["build", "-Doptimize=ReleaseFast"])
            .current_dir(zig_path)
            .status();
    }

    let lib_dir = std::env::current_dir().unwrap().join(zig_path).join("zig-out/lib");
    let static_lib = lib_dir.join("libhdc.a");
    if static_lib.exists() {
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=hdc");
    }

    println!("cargo:rerun-if-changed={}/src", zig_path);
}
