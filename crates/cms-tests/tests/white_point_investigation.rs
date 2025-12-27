//! White Point Investigation
//!
//! Check the white point handling between moxcms and skcms.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Check SM245B white point
#[test]
fn check_sm245b_white_point() {
    eprintln!("\n=== SM245B White Point Investigation ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    eprintln!("SM245B white point: {:?}", profile.white_point);

    // Reference white points
    eprintln!("\nReference white points:");
    eprintln!("  D50: [0.9642, 1.0, 0.8251]");
    eprintln!("  D65: [0.9505, 1.0, 1.0888]");

    // Check if SM245B white point is closer to D50 or D65
    let wp = profile.white_point;
    let d50 = [0.9642f64, 1.0, 0.8251];
    let d65 = [0.9505f64, 1.0, 1.0888];

    let dist_d50 = ((wp.x - d50[0]).powi(2) + (wp.y - d50[1]).powi(2) + (wp.z - d50[2]).powi(2)).sqrt();
    let dist_d65 = ((wp.x - d65[0]).powi(2) + (wp.y - d65[1]).powi(2) + (wp.z - d65[2]).powi(2)).sqrt();

    eprintln!("\nDistance to D50: {:.6}", dist_d50);
    eprintln!("Distance to D65: {:.6}", dist_d65);

    if dist_d50 < dist_d65 {
        eprintln!("=> SM245B white point is closer to D50");
    } else {
        eprintln!("=> SM245B white point is closer to D65");
    }

    // Check the colorants
    eprintln!("\nSM245B colorants (from profile):");
    eprintln!("  Red:   [{:.6}, {:.6}, {:.6}]", profile.red_colorant.x, profile.red_colorant.y, profile.red_colorant.z);
    eprintln!("  Green: [{:.6}, {:.6}, {:.6}]", profile.green_colorant.x, profile.green_colorant.y, profile.green_colorant.z);
    eprintln!("  Blue:  [{:.6}, {:.6}, {:.6}]", profile.blue_colorant.x, profile.blue_colorant.y, profile.blue_colorant.z);

    // Sum of colorants should equal white point for an ICC profile
    let sum_x = profile.red_colorant.x + profile.green_colorant.x + profile.blue_colorant.x;
    let sum_y = profile.red_colorant.y + profile.green_colorant.y + profile.blue_colorant.y;
    let sum_z = profile.red_colorant.z + profile.green_colorant.z + profile.blue_colorant.z;
    eprintln!("\nSum of colorants: [{:.6}, {:.6}, {:.6}]", sum_x, sum_y, sum_z);
    eprintln!("Profile white point: [{:.6}, {:.6}, {:.6}]", wp.x, wp.y, wp.z);

    if (sum_x - wp.x).abs() < 0.01 && (sum_y - wp.y).abs() < 0.01 && (sum_z - wp.z).abs() < 0.01 {
        eprintln!("=> Colorants sum to white point (as expected for ICC profile)");
    } else {
        eprintln!("=> WARNING: Colorants do NOT sum to white point!");
    }

    // Also check sRGB
    let srgb = moxcms::ColorProfile::new_srgb();
    eprintln!("\nsRGB white point: {:?}", srgb.white_point);
    eprintln!("sRGB colorants:");
    eprintln!("  Red:   [{:.6}, {:.6}, {:.6}]", srgb.red_colorant.x, srgb.red_colorant.y, srgb.red_colorant.z);
    eprintln!("  Green: [{:.6}, {:.6}, {:.6}]", srgb.green_colorant.x, srgb.green_colorant.y, srgb.green_colorant.z);
    eprintln!("  Blue:  [{:.6}, {:.6}, {:.6}]", srgb.blue_colorant.x, srgb.blue_colorant.y, srgb.blue_colorant.z);

    let srgb_sum_x = srgb.red_colorant.x + srgb.green_colorant.x + srgb.blue_colorant.x;
    let srgb_sum_y = srgb.red_colorant.y + srgb.green_colorant.y + srgb.blue_colorant.y;
    let srgb_sum_z = srgb.red_colorant.z + srgb.green_colorant.z + srgb.blue_colorant.z;
    eprintln!("\nsRGB colorants sum: [{:.6}, {:.6}, {:.6}]", srgb_sum_x, srgb_sum_y, srgb_sum_z);
}

/// Test what happens with white and black
#[test]
fn test_white_and_black() {
    eprintln!("\n=== White and Black Transform Test ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Test with skcms
    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
        let skcms_srgb = skcms_sys::srgb_profile();

        eprintln!("skcms SM245B -> sRGB:");

        // White
        let mut white_out = [0u8; 3];
        skcms_sys::transform(
            &[255u8, 255, 255],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut white_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [255, 255, 255] -> {:?}", white_out);

        // Black
        let mut black_out = [0u8; 3];
        skcms_sys::transform(
            &[0u8, 0, 0],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut black_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [0, 0, 0] -> {:?}", black_out);

        // 50% gray
        let mut gray_out = [0u8; 3];
        skcms_sys::transform(
            &[128u8, 128, 128],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut gray_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [128, 128, 128] -> {:?}", gray_out);
    }

    // Test with moxcms
    let mox_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let mox_transform = mox_profile.create_transform_8bit(
        moxcms::Layout::Rgb,
        &mox_srgb,
        moxcms::Layout::Rgb,
        moxcms::TransformOptions::default(),
    ).unwrap();

    eprintln!("\nmoxcms SM245B -> sRGB:");
    for input in [[255u8, 255, 255], [0, 0, 0], [128, 128, 128]] {
        let mut out = [0u8; 3];
        mox_transform.transform(&input, &mut out).unwrap();
        eprintln!("  {:?} -> {:?}", input, out);
    }

    eprintln!("\nNote: If white [255,255,255] doesn't map to [255,255,255],");
    eprintln!("the profiles have different gamuts or white points are handled differently.");
}

/// Examine what matrix moxcms is actually using
#[test]
fn examine_moxcms_matrix() {
    eprintln!("\n=== moxcms Matrix Examination ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let sm245b = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Get the transform matrix
    let matrix = sm245b.transform_matrix(&srgb);

    eprintln!("SM245B -> sRGB transform matrix:");
    for i in 0..3 {
        let row_sum: f64 = matrix.v[i][0] + matrix.v[i][1] + matrix.v[i][2];
        eprintln!("  [{:+.6}, {:+.6}, {:+.6}] (sum={:.6})",
            matrix.v[i][0], matrix.v[i][1], matrix.v[i][2], row_sum);
    }

    // Column sums
    let col0: f64 = matrix.v[0][0] + matrix.v[1][0] + matrix.v[2][0];
    let col1: f64 = matrix.v[0][1] + matrix.v[1][1] + matrix.v[2][1];
    let col2: f64 = matrix.v[0][2] + matrix.v[1][2] + matrix.v[2][2];
    eprintln!("\nColumn sums: [{:.6}, {:.6}, {:.6}]", col0, col1, col2);

    // What does the matrix do to [1,1,1] (linear white)?
    let white_out_x = matrix.v[0][0] + matrix.v[0][1] + matrix.v[0][2];
    let white_out_y = matrix.v[1][0] + matrix.v[1][1] + matrix.v[1][2];
    let white_out_z = matrix.v[2][0] + matrix.v[2][1] + matrix.v[2][2];
    eprintln!("\nMatrix * [1,1,1]: [{:.6}, {:.6}, {:.6}]", white_out_x, white_out_y, white_out_z);

    if (white_out_x - 1.0).abs() < 0.001 && (white_out_y - 1.0).abs() < 0.001 && (white_out_z - 1.0).abs() < 0.001 {
        eprintln!("=> Matrix preserves white (rows sum to 1.0)");
    } else {
        eprintln!("=> Matrix does NOT preserve white");
    }

    // The raw colorant matrices
    eprintln!("\nSM245B colorant matrix:");
    let sm_matrix = sm245b.colorant_matrix();
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]", sm_matrix.v[i][0], sm_matrix.v[i][1], sm_matrix.v[i][2]);
    }

    eprintln!("\nsRGB colorant matrix:");
    let srgb_matrix = srgb.colorant_matrix();
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]", srgb_matrix.v[i][0], srgb_matrix.v[i][1], srgb_matrix.v[i][2]);
    }
}
