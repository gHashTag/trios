//! verify-task — Validates global invariants for TRIOS workspace
//! Usage: cargo run -p trios-tri --bin verify-task

use std::fs;
use std::path::Path;

fn main() {
    let mut pass = 0usize;
    let mut fail = 0usize;

    let check = |desc: &str, result: bool, p: &mut usize, f: &mut usize| {
        if result {
            println!("  CHECK: {desc} ... \x1b[32mPASS\x1b[0m");
            *p += 1;
        } else {
            println!("  CHECK: {desc} ... \x1b[31mFAIL\x1b[0m");
            *f += 1;
        }
    };

    println!("=== Verifying TRIOS Global Invariants ===\n");

    // I1: bg-sw.js exists
    check(
        "I1: bg-sw.js exists",
        Path::new("crates/trios-ext/extension/dist/bg-sw.js").exists(),
        &mut pass,
        &mut fail,
    );

    // I2: bootstrap.js exists
    check(
        "I2: bootstrap.js exists",
        Path::new("crates/trios-ext/extension/dist/bootstrap.js").exists(),
        &mut pass,
        &mut fail,
    );

    // I3: 0 WASM imports in SW (comments ok, imports not)
    {
        let bg = fs::read_to_string("crates/trios-ext/extension/dist/bg-sw.js")
            .unwrap_or_default();
        let has_wasm_import = bg.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .any(|l| l.contains("WebAssembly") || l.contains("import") && l.contains("wasm"));
        check("I3: 0 WASM imports in SW", !has_wasm_import, &mut pass, &mut fail);
    }

    // I4: 0 WebSocket in extension code
    {
        let mcp = fs::read_to_string("crates/trios-ext/src/mcp.rs").unwrap_or_default();
        check(
            "I4: 0 WebSocket in ext",
            !mcp.contains("WebSocket") && !mcp.contains("ws://"),
            &mut pass,
            &mut fail,
        );
    }

    // I5: Single extension tree
    check(
        "I5: Single ext tree",
        Path::new("crates/trios-ext/extension").is_dir(),
        &mut pass,
        &mut fail,
    );

    // I7: CSP wasm-unsafe-eval
    {
        let manifest = fs::read_to_string("crates/trios-ext/extension/manifest.json")
            .unwrap_or_default();
        check(
            "I7: CSP wasm-unsafe-eval",
            manifest.contains("wasm-unsafe-eval"),
            &mut pass,
            &mut fail,
        );
    }

    // I12: --target web output
    {
        let js = fs::read_to_string("crates/trios-ext/extension/dist/trios_ext.js")
            .unwrap_or_default();
        check(
            "I12: --target web output",
            js.contains("import.meta.url"),
            &mut pass,
            &mut fail,
        );
    }

    // I13: Accept header in mcp.rs
    {
        let mcp = fs::read_to_string("crates/trios-ext/src/mcp.rs").unwrap_or_default();
        check("I13: Accept header", mcp.contains("Accept"), &mut pass, &mut fail);
    }

    // I14: Chat REST endpoint
    {
        let mcp = fs::read_to_string("crates/trios-ext/src/mcp.rs").unwrap_or_default();
        check(
            "I14: Chat REST endpoint",
            mcp.contains("CHAT_HTTP_URL"),
            &mut pass,
            &mut fail,
        );
    }

    // Summary
    println!(
        "\n=== Results: \x1b[32m{pass} passed\x1b[0m, \x1b[31m{fail} failed\x1b[0m ==="
    );

    if fail > 0 {
        println!("\x1b[31mBLOCKED: {fail} invariant(s) violated. Do not merge.\x1b[0m");
        std::process::exit(1);
    } else {
        println!("\x1b[32mAll invariants hold. Safe to merge.\x1b[0m");
    }
}
