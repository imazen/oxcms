//! Transform Trace Test
//!
//! Manually trace through the transform steps to find where the bug occurs.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Manually trace the SM245B -> sRGB transform
#[test]
fn trace_sm245b_to_srgb() {
    eprintln!("\n=== Manual Transform Trace ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Step 1: Get the raw TRC LUT from SM245B
    if let Some(moxcms::ToneReprCurve::Lut(lut)) = &profile.red_trc {
        eprintln!("SM245B TRC LUT: {} entries", lut.len());

        // For input 128 (0.502 normalized):
        let input_8bit = 128u8;
        let input_norm = input_8bit as f32 / 255.0;
        eprintln!("\nInput: {} (normalized: {:.4})", input_8bit, input_norm);

        // Linear interpolation in the LUT
        let lut_pos = input_norm * (lut.len() - 1) as f32;
        let lower = lut_pos.floor() as usize;
        let upper = (lower + 1).min(lut.len() - 1);
        let frac = lut_pos - lower as f32;

        let lower_val = lut[lower] as f32 / 65535.0;
        let upper_val = lut[upper] as f32 / 65535.0;
        let linear_sm = lower_val * (1.0 - frac) + upper_val * frac;

        eprintln!("Step 1: SM245B linearization");
        eprintln!(
            "  LUT position: {:.2} (between {} and {})",
            lut_pos, lower, upper
        );
        eprintln!("  LUT[{}] = {} ({:.5})", lower, lut[lower], lower_val);
        eprintln!("  LUT[{}] = {} ({:.5})", upper, lut[upper], upper_val);
        eprintln!("  Interpolated linear: {:.5}", linear_sm);

        // Step 2: Matrix transformation (for gray, should be ~identity on values)
        let matrix = profile.transform_matrix(&srgb);
        eprintln!("\nStep 2: Matrix transformation");
        eprintln!("  For gray input, matrix sum per row should be ~1.0");

        // For gray, all three channels are the same, so:
        let linear_sm_64 = linear_sm as f64;
        let out_r = (matrix.v[0][0] * linear_sm_64
            + matrix.v[0][1] * linear_sm_64
            + matrix.v[0][2] * linear_sm_64) as f32;
        let out_g = (matrix.v[1][0] * linear_sm_64
            + matrix.v[1][1] * linear_sm_64
            + matrix.v[1][2] * linear_sm_64) as f32;
        let out_b = (matrix.v[2][0] * linear_sm_64
            + matrix.v[2][1] * linear_sm_64
            + matrix.v[2][2] * linear_sm_64) as f32;

        eprintln!(
            "  Linear in: [{:.5}, {:.5}, {:.5}]",
            linear_sm, linear_sm, linear_sm
        );
        eprintln!("  Linear out: [{:.5}, {:.5}, {:.5}]", out_r, out_g, out_b);

        // Step 3: sRGB gamma encoding
        eprintln!("\nStep 3: sRGB gamma encoding");
        fn srgb_gamma(linear: f32) -> f32 {
            if linear <= 0.0031308 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
        }

        let encoded_r = srgb_gamma(out_r);
        let encoded_g = srgb_gamma(out_g);
        let encoded_b = srgb_gamma(out_b);

        eprintln!(
            "  Encoded: [{:.5}, {:.5}, {:.5}]",
            encoded_r, encoded_g, encoded_b
        );
        eprintln!(
            "  8-bit: [{}, {}, {}]",
            (encoded_r * 255.0).round() as u8,
            (encoded_g * 255.0).round() as u8,
            (encoded_b * 255.0).round() as u8
        );

        // Compare with actual moxcms output
        let transform = profile
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        let mut mox_out = [0u8; 3];
        transform.transform(&[128, 128, 128], &mut mox_out).unwrap();
        eprintln!("\nActual moxcms output: {:?}", mox_out);

        // Compare with skcms
        if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
            let skcms_srgb = skcms_sys::srgb_profile();
            let mut skcms_out = [0u8; 3];
            skcms_sys::transform(
                &[128u8, 128, 128],
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut skcms_out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );
            eprintln!("skcms output: {:?}", skcms_out);
        }

        eprintln!("\n=== Analysis ===");
        let expected_8bit = (encoded_r * 255.0).round() as u8;
        if mox_out[0] != expected_8bit {
            eprintln!(
                "BUG: moxcms output {} != expected {} from manual calculation",
                mox_out[0], expected_8bit
            );
            eprintln!("This suggests the bug is in the lookup table construction, not the math");
        } else {
            eprintln!("moxcms matches manual calculation, but differs from skcms");
            eprintln!("This suggests different TRC interpretation between moxcms and skcms");
        }
    }
}

/// Test what sRGB linearization looks like
#[test]
fn test_srgb_linearization() {
    eprintln!("\n=== sRGB Linearization Check ===\n");

    // sRGB forward (gamma to linear):
    // if x <= 0.04045: linear = x / 12.92
    // else: linear = ((x + 0.055) / 1.055)^2.4

    fn srgb_linearize(encoded: f32) -> f32 {
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    }

    fn srgb_gamma(linear: f32) -> f32 {
        if linear <= 0.0031308 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        }
    }

    eprintln!("sRGB linearization:");
    for v in [0, 64, 128, 192, 255] {
        let encoded = v as f32 / 255.0;
        let linear = srgb_linearize(encoded);
        eprintln!("  {} ({:.4}) -> {:.5}", v, encoded, linear);
    }

    // Now test round-trip
    eprintln!("\nsRGB round-trip (linearize then gamma):");
    for v in [0, 64, 128, 192, 255] {
        let encoded = v as f32 / 255.0;
        let linear = srgb_linearize(encoded);
        let back = srgb_gamma(linear);
        let back_8bit = (back * 255.0).round() as u8;
        eprintln!(
            "  {} -> linear {:.5} -> {:.4} -> {}",
            v, linear, back, back_8bit
        );
    }
}

/// Compare SM245B TRC with sRGB TRC
#[test]
fn compare_sm245b_vs_srgb_trc() {
    eprintln!("\n=== SM245B vs sRGB TRC Comparison ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    fn srgb_linearize(encoded: f32) -> f32 {
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    }

    if let Some(moxcms::ToneReprCurve::Lut(lut)) = &profile.red_trc {
        eprintln!("Comparing linearization curves:");
        eprintln!("Input | SM245B linear | sRGB linear | diff");
        eprintln!("------|---------------|-------------|-----");

        for v in [0, 32, 64, 96, 128, 160, 192, 224, 255] {
            let encoded = v as f32 / 255.0;

            // SM245B linearization (interpolate in LUT)
            let lut_pos = encoded * (lut.len() - 1) as f32;
            let lower = lut_pos.floor() as usize;
            let upper = (lower + 1).min(lut.len() - 1);
            let frac = lut_pos - lower as f32;
            let sm_linear = (lut[lower] as f32 * (1.0 - frac) + lut[upper] as f32 * frac) / 65535.0;

            // sRGB linearization
            let srgb_linear = srgb_linearize(encoded);

            let diff = (sm_linear - srgb_linear) * 1000.0; // in milli-units
            eprintln!(
                " {:3}  |    {:.5}    |   {:.5}   | {:+.3}m",
                v, sm_linear, srgb_linear, diff
            );
        }

        eprintln!("\nNote: diff in milli-units (m)");
        eprintln!("Positive diff = SM245B is brighter (less gamma)");
        eprintln!("Negative diff = SM245B is darker (more gamma)");
    }
}
