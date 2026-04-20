use trios_fpga::*;

#[test]
fn fpga_modules_no_duplicates() {
    let mut seen = std::collections::HashSet::new();
    for m in FPGA_MODULES {
        assert!(seen.insert(m), "duplicate module: {}", m);
    }
}

#[test]
fn fpga_modules_known_count() {
    assert_eq!(FPGA_MODULES.len(), 36, "36 FPGA modules expected");
}

#[test]
fn fpga_modules_contains_core() {
    assert!(FPGA_MODULES.contains(&"mac"), "MAC is core");
    assert!(FPGA_MODULES.contains(&"uart"), "UART is core");
    assert!(FPGA_MODULES.contains(&"spi"), "SPI is core");
    assert!(FPGA_MODULES.contains(&"bridge"), "Bridge is core");
    assert!(FPGA_MODULES.contains(&"top_level"), "Top level is core");
}

#[test]
fn fpga_modules_contains_accelerators() {
    assert!(FPGA_MODULES.contains(&"gf16_accel"), "GF16 accelerator");
    assert!(FPGA_MODULES.contains(&"ternary_isa"), "Ternary ISA");
}

#[test]
fn fpga_modules_contains_eda() {
    assert!(FPGA_MODULES.contains(&"placement"));
    assert!(FPGA_MODULES.contains(&"router"));
    assert!(FPGA_MODULES.contains(&"cts"));
    assert!(FPGA_MODULES.contains(&"timing"));
    assert!(FPGA_MODULES.contains(&"power"));
    assert!(FPGA_MODULES.contains(&"dft"));
}

#[test]
fn fpga_modules_contains_verification() {
    assert!(FPGA_MODULES.contains(&"formal"));
    assert!(FPGA_MODULES.contains(&"lint"));
    assert!(FPGA_MODULES.contains(&"coverage"));
    assert!(FPGA_MODULES.contains(&"cdc"));
}
