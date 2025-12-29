//! SIMD-optimized gamma/transfer function operations
//!
//! Transfer functions (TRCs) are applied to every pixel channel.
//! Batch processing allows better SIMD utilization.

use multiversion::multiversion;

/// Apply simple gamma to a batch of values
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn apply_gamma_batch(input: &[f64], output: &mut [f64], gamma: f64) {
    assert!(output.len() >= input.len());

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        *out = inp.clamp(0.0, 1.0).powf(gamma);
    }
}

/// Apply simple gamma to f32 batch (more SIMD-friendly)
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn apply_gamma_batch_f32(input: &[f32], output: &mut [f32], gamma: f32) {
    assert!(output.len() >= input.len());

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        *out = inp.clamp(0.0, 1.0).powf(gamma);
    }
}

/// Apply sRGB decode (encoded → linear) to a batch
///
/// sRGB transfer function:
/// - Linear segment: Y = X / 12.92 for X <= 0.04045
/// - Power segment: Y = ((X + 0.055) / 1.055)^2.4 for X > 0.04045
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn apply_srgb_decode_batch(input: &[f64], output: &mut [f64]) {
    assert!(output.len() >= input.len());

    const THRESHOLD: f64 = 0.04045;
    const LINEAR_SCALE: f64 = 1.0 / 12.92;
    const POWER_OFFSET: f64 = 0.055;
    const POWER_SCALE: f64 = 1.0 / 1.055;
    const POWER_EXP: f64 = 2.4;

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        let x = inp.clamp(0.0, 1.0);
        *out = if x <= THRESHOLD {
            x * LINEAR_SCALE
        } else {
            ((x + POWER_OFFSET) * POWER_SCALE).powf(POWER_EXP)
        };
    }
}

/// Apply sRGB encode (linear → encoded) to a batch
///
/// Inverse sRGB transfer function:
/// - Linear segment: Y = X * 12.92 for X <= 0.0031308
/// - Power segment: Y = 1.055 * X^(1/2.4) - 0.055 for X > 0.0031308
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn apply_srgb_encode_batch(input: &[f64], output: &mut [f64]) {
    assert!(output.len() >= input.len());

    const THRESHOLD: f64 = 0.0031308;
    const LINEAR_SCALE: f64 = 12.92;
    const POWER_SCALE: f64 = 1.055;
    const POWER_OFFSET: f64 = 0.055;
    const POWER_EXP: f64 = 1.0 / 2.4;

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        let x = inp.clamp(0.0, 1.0);
        *out = if x <= THRESHOLD {
            x * LINEAR_SCALE
        } else {
            POWER_SCALE * x.powf(POWER_EXP) - POWER_OFFSET
        };
    }
}

/// Apply sRGB decode to f32 batch
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn apply_srgb_decode_batch_f32(input: &[f32], output: &mut [f32]) {
    assert!(output.len() >= input.len());

    const THRESHOLD: f32 = 0.04045;
    const LINEAR_SCALE: f32 = 1.0 / 12.92;
    const POWER_OFFSET: f32 = 0.055;
    const POWER_SCALE: f32 = 1.0 / 1.055;
    const POWER_EXP: f32 = 2.4;

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        let x = inp.clamp(0.0, 1.0);
        *out = if x <= THRESHOLD {
            x * LINEAR_SCALE
        } else {
            ((x + POWER_OFFSET) * POWER_SCALE).powf(POWER_EXP)
        };
    }
}

/// Apply a lookup table to a batch of values
///
/// This is used for TRC curves that are stored as tables.
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn apply_lut1d_batch(input: &[f64], output: &mut [f64], lut: &[f64]) {
    assert!(output.len() >= input.len());

    if lut.is_empty() {
        output[..input.len()].copy_from_slice(input);
        return;
    }

    let lut_max = (lut.len() - 1) as f64;

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        let x = inp.clamp(0.0, 1.0);
        let pos = x * lut_max;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f64;

        if idx >= lut.len() - 1 {
            *out = lut[lut.len() - 1];
        } else {
            *out = lut[idx] + frac * (lut[idx + 1] - lut[idx]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamma_batch() {
        let input = [0.0, 0.25, 0.5, 0.75, 1.0];
        let mut output = [0.0; 5];

        apply_gamma_batch(&input, &mut output, 2.2);

        // Check that 0 and 1 are preserved
        assert!((output[0] - 0.0).abs() < 1e-10);
        assert!((output[4] - 1.0).abs() < 1e-10);

        // Check that 0.5^2.2 ≈ 0.2176
        assert!((output[2] - 0.5_f64.powf(2.2)).abs() < 1e-10);
    }

    #[test]
    fn test_srgb_decode_batch() {
        let input = [0.0, 0.04045, 0.5, 1.0];
        let mut output = [0.0; 4];

        apply_srgb_decode_batch(&input, &mut output);

        // Black stays black
        assert!((output[0] - 0.0).abs() < 1e-10);
        // White stays white
        assert!((output[3] - 1.0).abs() < 1e-10);

        // Threshold point
        assert!((output[1] - 0.04045 / 12.92).abs() < 1e-10);
    }

    #[test]
    fn test_srgb_roundtrip() {
        let mut linear = [0.0; 256];
        let mut encoded = [0.0; 256];
        let mut roundtrip = [0.0; 256];

        // Create input values
        let input: Vec<f64> = (0..256).map(|i| i as f64 / 255.0).collect();

        apply_srgb_decode_batch(&input, &mut linear);
        apply_srgb_encode_batch(&linear, &mut encoded);

        // Roundtrip should be close to original
        for i in 0..256 {
            assert!(
                (encoded[i] - input[i]).abs() < 1e-10,
                "Mismatch at {}: {} vs {}",
                i,
                input[i],
                encoded[i]
            );
        }
    }

    #[test]
    fn test_lut1d_batch() {
        // Linear LUT
        let lut: Vec<f64> = (0..256).map(|i| i as f64 / 255.0).collect();
        let input = [0.0, 0.25, 0.5, 0.75, 1.0];
        let mut output = [0.0; 5];

        apply_lut1d_batch(&input, &mut output, &lut);

        // Should be approximately identity
        for i in 0..5 {
            assert!((output[i] - input[i]).abs() < 0.01);
        }
    }
}
