// Simple GF16 test using direct FFI linking
use std::os::raw::c_float;

#[link(name = "goldenfloat")]
extern "C" {
    fn gf16_from_f32(x: c_float) -> u16;
    fn gf16_to_f32(g: u16) -> c_float;
    fn gf16_add(a: u16, b: u16) -> u16;
    fn gf16_mul(a: u16, b: u16) -> u16;
    fn gf16_phi_quantize(x: c_float) -> u16;
    fn gf16_phi_dequantize(g: u16) -> c_float;
}

fn main() {
    println!("=== GF16 Conversion Test ===");
    println!("Value | GF16 (hex) | GF16 (f32) | Error");
    println!("------|------------|------------|-------");

    let test_values = [0.0_f32, 1.0, -1.0, 3.14159, 2.71828, 0.5, 100.0, -0.01];
    let mut max_error = 0.0_f32;

    for val in test_values {
        unsafe {
            let gf = gf16_from_f32(val);
            let back = gf16_to_f32(gf);
            let err = (val - back).abs() / (val.abs().max(1e-6));
            max_error = max_error.max(err);
            println!("{:6.4} | 0x{:04x}    | {:10.6} | {:.6}%", val, gf, back, err * 100.0);
        }
    }

    println!("\nMax relative error: {:.6}%", max_error * 100.0);

    // Test arithmetic
    println!("\n=== GF16 Arithmetic Test ===");
    let (a, b) = (2.0_f32, 3.0_f32);
    unsafe {
        let (gf_a, gf_b) = (gf16_from_f32(a), gf16_from_f32(b));
        let sum = gf16_to_f32(gf16_add(gf_a, gf_b));
        let prod = gf16_to_f32(gf16_mul(gf_a, gf_b));
        println!("{} + {} = {} (error: {:.6}%)", a, b, sum, ((a + b) - sum).abs() / (a + b) * 100.0);
        println!("{} * {} = {} (error: {:.6}%)", a, b, prod, ((a * b) - prod).abs() / (a * b) * 100.0);
    }

    // Test φ-quantization
    println!("\n=== GF16 φ-Quantization Test ===");
    let phi_val: f64 = 1.618033988749895;
    unsafe {
        let quantized = gf16_phi_quantize(phi_val as f32);
        let dequantized = gf16_phi_dequantize(quantized) as f64;
        let error = (phi_val - dequantized).abs() / phi_val;
        println!("φ = {}", phi_val);
        println!("φ-quantized = 0x{:04x}", quantized);
        println!("φ-dequantized = {}", dequantized);
        println!("Error: {:.6}%", error * 100.0);
    }

    println!("\n=== Test Complete ===");
}
