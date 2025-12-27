//! Chromatic Adaptation Debug Test

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

#[test]
fn debug_chromatic_adaptation() {
    eprintln!("\n=== Chromatic Adaptation Debug ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Print colorant sums
    let sum_x = profile.red_colorant.x + profile.green_colorant.x + profile.blue_colorant.x;
    let sum_y = profile.red_colorant.y + profile.green_colorant.y + profile.blue_colorant.y;
    let sum_z = profile.red_colorant.z + profile.green_colorant.z + profile.blue_colorant.z;
    eprintln!(
        "SM245B colorant sum: [{:.6}, {:.6}, {:.6}]",
        sum_x, sum_y, sum_z
    );

    // D50 reference
    let d50 = moxcms::Chromaticity::D50.to_xyzd();
    eprintln!("D50: [{:.6}, {:.6}, {:.6}]", d50.x, d50.y, d50.z);

    // Check differences
    let dx = (sum_x - d50.x).abs();
    let dy = (sum_y - d50.y).abs();
    let dz = (sum_z - d50.z).abs();
    eprintln!("Diff from D50: [{:.6}, {:.6}, {:.6}]", dx, dy, dz);

    // Get the transform matrices
    let sm_xyz_matrix = profile.rgb_to_xyz_matrix();
    let srgb_xyz_matrix = srgb.rgb_to_xyz_matrix();

    eprintln!("\nSM245B rgb_to_xyz_matrix:");
    for i in 0..3 {
        eprintln!(
            "  [{:.6}, {:.6}, {:.6}]",
            sm_xyz_matrix.v[i][0], sm_xyz_matrix.v[i][1], sm_xyz_matrix.v[i][2]
        );
    }

    eprintln!("\nsRGB rgb_to_xyz_matrix:");
    for i in 0..3 {
        eprintln!(
            "  [{:.6}, {:.6}, {:.6}]",
            srgb_xyz_matrix.v[i][0], srgb_xyz_matrix.v[i][1], srgb_xyz_matrix.v[i][2]
        );
    }

    // The transform matrix used
    let transform_matrix = profile.transform_matrix(&srgb);
    eprintln!("\nTransform matrix (SM245B -> sRGB):");
    for i in 0..3 {
        eprintln!(
            "  [{:.6}, {:.6}, {:.6}]",
            transform_matrix.v[i][0], transform_matrix.v[i][1], transform_matrix.v[i][2]
        );
    }

    // Test a transform
    let mox_transform = profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let mut out = [0u8; 3];
    mox_transform.transform(&[128, 128, 128], &mut out).unwrap();
    eprintln!("\nmoxcms [128,128,128] -> {:?}", out);

    // skcms comparison
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
        eprintln!("skcms [128,128,128] -> {:?}", skcms_out);
    }
}

/// Test to understand what skcms is actually computing
#[test]
fn compare_skcms_internal_matrix() {
    eprintln!("\n=== skcms Internal Matrix Comparison ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Parse with skcms and examine its internal state
    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
        let skcms_srgb = skcms_sys::srgb_profile();

        // Test pure colors to understand the matrix
        let test_inputs: &[[u8; 3]] = &[
            [255, 0, 0], // Pure red
            [0, 255, 0], // Pure green
            [0, 0, 255], // Pure blue
            [128, 0, 0], // Half red
            [0, 128, 0], // Half green
            [0, 0, 128], // Half blue
        ];

        eprintln!("skcms SM245B -> sRGB transforms:");
        for input in test_inputs {
            let mut out = [0u8; 3];
            skcms_sys::transform(
                input,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );
            eprintln!("  {:?} -> {:?}", input, out);
        }

        // Same tests with moxcms
        let mox_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
        let mox_srgb = moxcms::ColorProfile::new_srgb();
        let mox_transform = mox_profile
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &mox_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        eprintln!("\nmoxcms SM245B -> sRGB transforms:");
        for input in test_inputs {
            let mut out = [0u8; 3];
            mox_transform.transform(input, &mut out).unwrap();
            eprintln!("  {:?} -> {:?}", input, out);
        }
    }
}
