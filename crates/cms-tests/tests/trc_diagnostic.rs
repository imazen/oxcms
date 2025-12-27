//! TRC Diagnostic Test
//!
//! Deep investigation into the TRC curves themselves to find the +20 bug.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Test the raw TRC curve values from SM245B
#[test]
fn diagnose_sm245b_trc() {
    eprintln!("\n=== SM245B TRC Diagnostic ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    eprintln!("SM245B profile info:");
    eprintln!("  Is matrix-shaper: {}", profile.is_matrix_shaper());
    eprintln!("  Color space: {:?}", profile.color_space);
    eprintln!("  PCS: {:?}", profile.pcs);

    // Get the colorant matrices
    let sm_matrix = profile.colorant_matrix();
    let srgb_matrix = srgb.colorant_matrix();

    eprintln!("\nSM245B colorant matrix:");
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]",
            sm_matrix.v[i][0], sm_matrix.v[i][1], sm_matrix.v[i][2]);
    }

    eprintln!("\nsRGB colorant matrix:");
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]",
            srgb_matrix.v[i][0], srgb_matrix.v[i][1], srgb_matrix.v[i][2]);
    }

    // Create transforms in both directions and test specific values
    let sm_to_srgb = profile
        .create_transform_f32(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let srgb_to_sm = srgb
        .create_transform_f32(
            moxcms::Layout::Rgb,
            &profile,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    // Also test identity transforms
    let sm_identity = profile
        .create_transform_f32(
            moxcms::Layout::Rgb,
            &profile,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let srgb_identity = srgb
        .create_transform_f32(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("\n=== Float Transform Results (0.0-1.0 scale) ===");
    eprintln!("Input | SM->sRGB | sRGB->SM | SM->SM | sRGB->sRGB");
    eprintln!("------|----------|----------|--------|----------");

    for v in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
        let input = [v, v, v];

        let mut sm_to_srgb_out = [0.0f32; 3];
        let mut srgb_to_sm_out = [0.0f32; 3];
        let mut sm_identity_out = [0.0f32; 3];
        let mut srgb_identity_out = [0.0f32; 3];

        sm_to_srgb.transform(&input, &mut sm_to_srgb_out).unwrap();
        srgb_to_sm.transform(&input, &mut srgb_to_sm_out).unwrap();
        sm_identity.transform(&input, &mut sm_identity_out).unwrap();
        srgb_identity.transform(&input, &mut srgb_identity_out).unwrap();

        eprintln!(
            "{:.2}  | {:.4}   | {:.4}   | {:.4} | {:.4}",
            v,
            sm_to_srgb_out[0],
            srgb_to_sm_out[0],
            sm_identity_out[0],
            srgb_identity_out[0]
        );
    }

    // Compare with lcms2
    eprintln!("\n=== Comparison with lcms2 (f32) ===");

    let lcms2_sm = lcms2::Profile::new_icc(&data).unwrap();
    let lcms2_srgb = lcms2::Profile::new_srgb();

    // Note: lcms2 doesn't have a direct f32 format, use RGB_8 and convert
    let lcms2_sm_to_srgb = lcms2::Transform::new(
        &lcms2_sm,
        lcms2::PixelFormat::RGB_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    ).unwrap();

    eprintln!("8-bit comparison (input 0-255):");
    eprintln!("Input | moxcms | lcms2 | diff");
    eprintln!("------|--------|-------|-----");

    let transform_8 = profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    for v in [0u8, 64, 128, 192, 255] {
        let input = [v, v, v];

        let mut mox_out = [0u8; 3];
        let mut lcms_out = [0u8; 3];

        transform_8.transform(&input, &mut mox_out).unwrap();
        lcms2_sm_to_srgb.transform_pixels(&input, &mut lcms_out);

        let diff = mox_out[0] as i32 - lcms_out[0] as i32;
        eprintln!(" {:3}  |  {:3}   |  {:3}  | {:+3}", v, mox_out[0], lcms_out[0], diff);
    }

    // Test round-trip to isolate where the error comes from
    eprintln!("\n=== Round-trip Test ===");
    eprintln!("Testing: sRGB -> SM245B -> sRGB vs identity");

    let srgb_to_sm_8 = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &profile,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let sm_to_srgb_8 = profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("Input | after sRGB->SM | after SM->sRGB | round-trip error");
    eprintln!("------|----------------|----------------|------------------");

    for v in [0u8, 64, 128, 192, 255] {
        let input = [v, v, v];
        let mut intermediate = [0u8; 3];
        let mut final_out = [0u8; 3];

        srgb_to_sm_8.transform(&input, &mut intermediate).unwrap();
        sm_to_srgb_8.transform(&intermediate, &mut final_out).unwrap();

        let error = final_out[0] as i32 - v as i32;
        eprintln!(" {:3}  |      {:3}       |      {:3}       |       {:+3}",
            v, intermediate[0], final_out[0], error);
    }
}

/// Compare moxcms matrix calculation with reference
#[test]
fn diagnose_transform_matrix() {
    eprintln!("\n=== Transform Matrix Diagnostic ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let sm245b = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Get the transform matrix that moxcms calculates
    let transform_matrix = sm245b.transform_matrix(&srgb);

    eprintln!("SM245B -> sRGB transform matrix:");
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]",
            transform_matrix.v[i][0], transform_matrix.v[i][1], transform_matrix.v[i][2]);
    }

    // For a neutral color (gray), the matrix should produce roughly equal R, G, B
    // If SM245B has D65 white point, gray in SM245B should stay gray in sRGB
    eprintln!("\nMatrix * [1,1,1] (white):");
    let white_out = [
        transform_matrix.v[0][0] + transform_matrix.v[0][1] + transform_matrix.v[0][2],
        transform_matrix.v[1][0] + transform_matrix.v[1][1] + transform_matrix.v[1][2],
        transform_matrix.v[2][0] + transform_matrix.v[2][1] + transform_matrix.v[2][2],
    ];
    eprintln!("  [{:.6}, {:.6}, {:.6}]", white_out[0], white_out[1], white_out[2]);

    // The inverse direction
    let inverse_matrix = srgb.transform_matrix(&sm245b);
    eprintln!("\nsRGB -> SM245B transform matrix:");
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]",
            inverse_matrix.v[i][0], inverse_matrix.v[i][1], inverse_matrix.v[i][2]);
    }
}

/// Test to see if the TRC curves themselves are being handled correctly
#[test]
fn diagnose_trc_tables() {
    eprintln!("\n=== TRC Table Diagnostic ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    // Parse with skcms to get the raw TRC data
    let data = std::fs::read(&profile_path).unwrap();

    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
        eprintln!("skcms parsed SM245B successfully");

        // The skcms profile has TRC information we can examine
        // For now, let's just compare transform outputs

        let skcms_srgb = skcms_sys::srgb_profile();

        eprintln!("\nskcms transform outputs:");
        for v in [0u8, 64, 128, 192, 255] {
            let color = [v, v, v];
            let mut out = [0u8; 3];

            skcms_sys::transform(
                &color,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );

            eprintln!("  {} -> {}", v, out[0]);
        }
    }

    // Now compare with qcms
    if let Some(qcms_profile) = qcms::Profile::new_from_slice(&data, false) {
        let qcms_srgb = qcms::Profile::new_sRGB();

        if let Some(qcms_transform) = qcms::Transform::new(
            &qcms_profile,
            &qcms_srgb,
            qcms::DataType::RGB8,
            qcms::Intent::Perceptual,
        ) {
            eprintln!("\nqcms transform outputs:");
            for v in [0u8, 64, 128, 192, 255] {
                let mut color = vec![v, v, v];
                qcms_transform.apply(&mut color);
                eprintln!("  {} -> {}", v, color[0]);
            }
        }
    }

    // And moxcms
    let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let moxcms_srgb = moxcms::ColorProfile::new_srgb();

    let moxcms_transform = moxcms_profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("\nmoxcms transform outputs:");
    for v in [0u8, 64, 128, 192, 255] {
        let color = [v, v, v];
        let mut out = [0u8; 3];
        moxcms_transform.transform(&color, &mut out).unwrap();
        eprintln!("  {} -> {}", v, out[0]);
    }

    // Summary
    eprintln!("\n=== Summary ===");
    eprintln!("If moxcms outputs are close to input (identity-like),");
    eprintln!("but skcms/qcms outputs are different, the TRC is not being applied.");
}
