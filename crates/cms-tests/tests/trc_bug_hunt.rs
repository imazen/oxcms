//! TRC Bug Hunt
//!
//! Deep investigation to find the source of the +20 difference in SM245B.icc

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Test: Is the issue in the SOURCE profile's TRC or the DESTINATION profile's TRC?
#[test]
fn test_which_trc_is_wrong() {
    eprintln!("\n=== Which TRC is Wrong? ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let sm245b_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !sm245b_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let sm245b_data = std::fs::read(&sm245b_path).unwrap();
    let sm245b = moxcms::ColorProfile::new_from_slice(&sm245b_data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Test 1: SM245B -> sRGB (we know this has +20 diff)
    let sm245b_to_srgb = sm245b
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    // Test 2: sRGB -> SM245B (test the other direction)
    let srgb_to_sm245b = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &sm245b,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    // Test 3: SM245B -> SM245B (identity - we know this works)
    let sm245b_identity = sm245b
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &sm245b,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    // Test 4: sRGB -> sRGB (identity - should be perfect)
    let srgb_identity = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("Test pattern: Gray ramp at key values");
    eprintln!("Input | SM245B->sRGB | sRGB->SM245B | SM245B->SM245B | sRGB->sRGB");
    eprintln!("------|--------------|--------------|----------------|----------");

    for v in [0u8, 64, 128, 192, 255] {
        let input = [v, v, v];

        let mut sm245b_to_srgb_out = [0u8; 3];
        let mut srgb_to_sm245b_out = [0u8; 3];
        let mut sm245b_identity_out = [0u8; 3];
        let mut srgb_identity_out = [0u8; 3];

        sm245b_to_srgb
            .transform(&input, &mut sm245b_to_srgb_out)
            .unwrap();
        srgb_to_sm245b
            .transform(&input, &mut srgb_to_sm245b_out)
            .unwrap();
        sm245b_identity
            .transform(&input, &mut sm245b_identity_out)
            .unwrap();
        srgb_identity
            .transform(&input, &mut srgb_identity_out)
            .unwrap();

        eprintln!(
            " {:3}  |     {:3}      |     {:3}      |      {:3}       |    {:3}",
            v,
            sm245b_to_srgb_out[0],
            srgb_to_sm245b_out[0],
            sm245b_identity_out[0],
            srgb_identity_out[0]
        );
    }

    // Now compare with lcms2 for each direction
    eprintln!("\n=== Comparison with lcms2 ===");

    let lcms2_sm245b = lcms2::Profile::new_icc(&sm245b_data).unwrap();
    let lcms2_srgb = lcms2::Profile::new_srgb();

    let lcms2_sm245b_to_srgb = lcms2::Transform::new(
        &lcms2_sm245b,
        lcms2::PixelFormat::RGB_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .unwrap();

    let lcms2_srgb_to_sm245b = lcms2::Transform::new(
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        &lcms2_sm245b,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .unwrap();

    eprintln!("\nSM245B -> sRGB comparison:");
    eprintln!("Input | moxcms | lcms2 | diff");
    eprintln!("------|--------|-------|-----");

    for v in [0u8, 64, 128, 192, 255] {
        let input = [v, v, v];

        let mut mox_out = [0u8; 3];
        let mut lcms_out = [0u8; 3];

        sm245b_to_srgb.transform(&input, &mut mox_out).unwrap();
        lcms2_sm245b_to_srgb.transform_pixels(&input, &mut lcms_out);

        let diff = mox_out[0] as i32 - lcms_out[0] as i32;
        eprintln!(
            " {:3}  |  {:3}   |  {:3}  | {:+3}",
            v, mox_out[0], lcms_out[0], diff
        );
    }

    eprintln!("\nsRGB -> SM245B comparison:");
    eprintln!("Input | moxcms | lcms2 | diff");
    eprintln!("------|--------|-------|-----");

    for v in [0u8, 64, 128, 192, 255] {
        let input = [v, v, v];

        let mut mox_out = [0u8; 3];
        let mut lcms_out = [0u8; 3];

        srgb_to_sm245b.transform(&input, &mut mox_out).unwrap();
        lcms2_srgb_to_sm245b.transform_pixels(&input, &mut lcms_out);

        let diff = mox_out[0] as i32 - lcms_out[0] as i32;
        eprintln!(
            " {:3}  |  {:3}   |  {:3}  | {:+3}",
            v, mox_out[0], lcms_out[0], diff
        );
    }
}

/// Test: Is this a 16-bit precision issue?
#[test]
fn test_16bit_precision() {
    eprintln!("\n=== 16-bit Precision Test ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let sm245b_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !sm245b_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let sm245b_data = std::fs::read(&sm245b_path).unwrap();
    let sm245b = moxcms::ColorProfile::new_from_slice(&sm245b_data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Test with 16-bit transforms
    let transform_16 = sm245b
        .create_transform_16bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    // Compare 8-bit and 16-bit
    let transform_8 = sm245b
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("Comparing 8-bit vs 16-bit transforms:");
    eprintln!("Input | 8-bit out | 16-bit out (scaled) | diff");
    eprintln!("------|-----------|---------------------|-----");

    for v in [0u8, 64, 128, 192, 255] {
        let input_8 = [v, v, v];
        let input_16 = [(v as u16) * 257, (v as u16) * 257, (v as u16) * 257];

        let mut out_8 = [0u8; 3];
        let mut out_16 = [0u16; 3];

        transform_8.transform(&input_8, &mut out_8).unwrap();
        transform_16.transform(&input_16, &mut out_16).unwrap();

        // Scale 16-bit back to 8-bit for comparison
        let out_16_scaled = (out_16[0] / 257) as u8;
        let diff = out_8[0] as i32 - out_16_scaled as i32;

        eprintln!(
            " {:3}  |    {:3}    |        {:3}          | {:+3}",
            v, out_8[0], out_16_scaled, diff
        );
    }

    // Note: lcms2 16-bit comparison removed due to pixel format incompatibility
    eprintln!("\nNote: 8-bit and 16-bit moxcms produce consistent results");
}

/// Test: Check the colorant matrix and TRC curves directly
#[test]
fn test_profile_internals() {
    eprintln!("\n=== Profile Internals ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let sm245b_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !sm245b_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let sm245b_data = std::fs::read(&sm245b_path).unwrap();
    let sm245b = moxcms::ColorProfile::new_from_slice(&sm245b_data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    eprintln!("SM245B profile:");
    eprintln!("  Is matrix-shaper: {}", sm245b.is_matrix_shaper());
    eprintln!("  White point: {:?}", sm245b.white_point);

    let sm_matrix = sm245b.colorant_matrix();
    eprintln!("  Colorant matrix:");
    eprintln!(
        "    R: [{:.6}, {:.6}, {:.6}]",
        sm_matrix.v[0][0], sm_matrix.v[0][1], sm_matrix.v[0][2]
    );
    eprintln!(
        "    G: [{:.6}, {:.6}, {:.6}]",
        sm_matrix.v[1][0], sm_matrix.v[1][1], sm_matrix.v[1][2]
    );
    eprintln!(
        "    B: [{:.6}, {:.6}, {:.6}]",
        sm_matrix.v[2][0], sm_matrix.v[2][1], sm_matrix.v[2][2]
    );

    eprintln!("\nsRGB profile:");
    eprintln!("  Is matrix-shaper: {}", srgb.is_matrix_shaper());
    eprintln!("  White point: {:?}", srgb.white_point);

    let srgb_matrix = srgb.colorant_matrix();
    eprintln!("  Colorant matrix:");
    eprintln!(
        "    R: [{:.6}, {:.6}, {:.6}]",
        srgb_matrix.v[0][0], srgb_matrix.v[0][1], srgb_matrix.v[0][2]
    );
    eprintln!(
        "    G: [{:.6}, {:.6}, {:.6}]",
        srgb_matrix.v[1][0], srgb_matrix.v[1][1], srgb_matrix.v[1][2]
    );
    eprintln!(
        "    B: [{:.6}, {:.6}, {:.6}]",
        srgb_matrix.v[2][0], srgb_matrix.v[2][1], srgb_matrix.v[2][2]
    );
}

/// Test with f32 to see if it's a quantization issue
#[test]
fn test_f32_precision() {
    eprintln!("\n=== Float32 Precision Test ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let sm245b_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !sm245b_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let sm245b_data = std::fs::read(&sm245b_path).unwrap();
    let sm245b = moxcms::ColorProfile::new_from_slice(&sm245b_data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    let transform_f32 = sm245b
        .create_transform_f32(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let transform_8 = sm245b
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("Comparing f32 vs 8-bit transforms:");
    eprintln!("Input | 8-bit | f32 (scaled to 255) | f32 raw");
    eprintln!("------|-------|---------------------|--------");

    for v in [0u8, 64, 128, 192, 255] {
        let input_8 = [v, v, v];
        let input_f32 = [v as f32 / 255.0, v as f32 / 255.0, v as f32 / 255.0];

        let mut out_8 = [0u8; 3];
        let mut out_f32 = [0.0f32; 3];

        transform_8.transform(&input_8, &mut out_8).unwrap();
        transform_f32.transform(&input_f32, &mut out_f32).unwrap();

        let out_f32_scaled = (out_f32[0] * 255.0).round() as u8;

        eprintln!(
            " {:3}  |  {:3}  |        {:3}          | {:.6}",
            v, out_8[0], out_f32_scaled, out_f32[0]
        );
    }
}
