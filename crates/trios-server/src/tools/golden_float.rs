//! Golden Float (GF16) tools module.
//!
//! Provides GF16 encoding/decoding and compression operations using φ-based representation.

use anyhow::{Context, Result};
use serde_json::Value;
use trios_golden_float::{compress_weights, GF16};

/// Dispatch golden float tools.
pub async fn dispatch(name: &str, input: &Value) -> Option<Result<Value>> {
    match name {
        "gf16_encode" => Some(gf16_encode(input).await),
        "gf16_decode" => Some(gf16_decode(input).await),
        "gf16_compress_weights" => Some(gf16_compress_weights(input).await),
        "phi_constant" => Some(phi_constant().await),
        _ => None,
    }
}

/// Encode an f32 value to GF16.
async fn gf16_encode(input: &Value) -> Result<Value> {
    let value = input
        .get("value")
        .and_then(|v| v.as_f64())
        .context("value (f64) required")? as f32;

    let gf16 = GF16::from_f32(value);
    Ok(serde_json::json!({
        "original": value,
        "encoded": gf16.to_bits(),
        "decoded_back": gf16.to_f32(),
    }))
}

/// Decode a GF16 value (from u16 bits) back to f32.
async fn gf16_decode(input: &Value) -> Result<Value> {
    let bits = input
        .get("encoded")
        .and_then(|v| v.as_u64())
        .context("encoded (u16 as u64) required")? as u16;

    let gf16 = GF16::from_bits(bits);
    Ok(serde_json::json!({
        "encoded": bits,
        "decoded": gf16.to_f32(),
    }))
}

/// Compress a slice of f32 weights into GF16 representation.
async fn gf16_compress_weights(input: &Value) -> Result<Value> {
    let weights: Vec<f32> = input
        .get("weights")
        .and_then(|v| v.as_array())
        .context("weights array required")?
        .iter()
        .filter_map(|v| v.as_f64())
        .map(|x| x as f32)
        .collect();

    if weights.is_empty() {
        anyhow::bail!("weights array cannot be empty");
    }

    let original_size = weights.len() * std::mem::size_of::<f32>();
    let compressed = compress_weights(&weights);
    let compressed_size = compressed.len() * std::mem::size_of::<u16>();
    let ratio = compressed_size as f32 / original_size as f32;

    Ok(serde_json::json!({
        "original_count": weights.len(),
        "compressed": compressed,
        "compression_ratio": ratio,
        "original_size_bytes": original_size,
        "compressed_size_bytes": compressed_size,
    }))
}

/// Return the golden ratio φ constant.
async fn phi_constant() -> Result<Value> {
    // φ = (1 + √5) / 2 ≈ 1.6180339887498948482
    const PHI: f64 = 1.618_033_988_749_895;
    Ok(serde_json::json!({
        "phi": PHI,
        "phi_approx": 1.618034,
        "inverse_phi": 1.0 / PHI,
        "phi_squared": PHI * PHI,
    }))
}
