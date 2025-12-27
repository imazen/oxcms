//! TRC Curve Investigation
//!
//! Deep investigation into TRC curve interpolation differences between
//! moxcms and browser CMS (skcms/qcms).

use skcms_sys::{skcms_AlphaFormat, skcms_PixelFormat};
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Investigate SM245B.icc TRC curve
/// This profile shows consistent +20 offset from browser consensus
#[test]
fn investigate_sm245b_trc() {
    eprintln!("\n=== SM245B.icc TRC Curve Investigation ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Parse with all CMS
    let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let qcms_profile = qcms::Profile::new_from_slice(&data, false).unwrap();
    let skcms_profile = skcms_sys::parse_icc_profile(&data).unwrap();
    let lcms2_profile = lcms2::Profile::new_icc(&data).unwrap();

    // Profile info
    eprintln!("Profile info:");
    eprintln!("  Color space: {:?}", moxcms_profile.color_space);
    eprintln!("  Is matrix-shaper: {}", moxcms_profile.is_matrix_shaper());
    eprintln!("  Version: {:?}", moxcms_profile.version());

    // Create transforms
    let moxcms_srgb = moxcms::ColorProfile::new_srgb();
    let moxcms_transform = moxcms_profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let qcms_srgb = qcms::Profile::new_sRGB();
    let qcms_transform = qcms::Transform::new(
        &qcms_profile,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )
    .unwrap();

    let lcms2_srgb = lcms2::Profile::new_srgb();
    let lcms2_transform = lcms2::Transform::new(
        &lcms2_profile,
        lcms2::PixelFormat::RGB_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .unwrap();

    let skcms_srgb = skcms_sys::srgb_profile();

    // Test the entire 0-255 range on neutral axis
    eprintln!("\nNeutral axis transform (input -> output):");
    eprintln!("Input | qcms | skcms | moxcms | lcms2 | mox-browser");
    eprintln!("------|------|-------|--------|-------|------------");

    let mut max_mox_diff = 0;
    let mut worst_input = 0;

    for v in (0..=255).step_by(16) {
        let color = [v as u8, v as u8, v as u8];

        // Transform with each CMS
        let mut qcms_out = color.to_vec();
        qcms_transform.apply(&mut qcms_out);

        let mut moxcms_out = [0u8; 3];
        moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

        let mut lcms2_out = [0u8; 3];
        lcms2_transform.transform_pixels(&color, &mut lcms2_out);

        let mut skcms_out = [0u8; 3];
        skcms_sys::transform(
            &color,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut skcms_out,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );

        let browser_avg = ((qcms_out[0] as i32 + skcms_out[0] as i32) / 2) as u8;
        let mox_diff = moxcms_out[0] as i32 - browser_avg as i32;

        if mox_diff.abs() > max_mox_diff {
            max_mox_diff = mox_diff.abs();
            worst_input = v;
        }

        eprintln!(
            "  {:3} |  {:3} |  {:3}  |  {:3}   |  {:3}  | {:+3}",
            v, qcms_out[0], skcms_out[0], moxcms_out[0], lcms2_out[0], mox_diff
        );
    }

    eprintln!("\nMax moxcms vs browser diff: {} at input {}", max_mox_diff, worst_input);

    // Now test high values in detail (where we saw the most difference)
    eprintln!("\nHigh value range (220-255):");
    eprintln!("Input | qcms | skcms | moxcms | lcms2 | mox-browser");
    eprintln!("------|------|-------|--------|-------|------------");

    for v in 220..=255 {
        let color = [v as u8, v as u8, v as u8];

        let mut qcms_out = color.to_vec();
        qcms_transform.apply(&mut qcms_out);

        let mut moxcms_out = [0u8; 3];
        moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

        let mut lcms2_out = [0u8; 3];
        lcms2_transform.transform_pixels(&color, &mut lcms2_out);

        let mut skcms_out = [0u8; 3];
        skcms_sys::transform(
            &color,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut skcms_out,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );

        let browser_avg = ((qcms_out[0] as i32 + skcms_out[0] as i32) / 2) as u8;
        let mox_diff = moxcms_out[0] as i32 - browser_avg as i32;

        eprintln!(
            "  {:3} |  {:3} |  {:3}  |  {:3}   |  {:3}  | {:+3}",
            v, qcms_out[0], skcms_out[0], moxcms_out[0], lcms2_out[0], mox_diff
        );
    }

    // Hypothesis: The difference is consistent across the range, suggesting
    // a TRC curve interpolation algorithm difference rather than a bug
    eprintln!("\n=== Analysis ===");
    eprintln!("1. moxcms consistently outputs BRIGHTER values than browsers");
    eprintln!("2. The difference increases with input value (more at highlights)");
    eprintln!("3. This suggests different TRC curve interpolation algorithm");
    eprintln!("4. Could be: linear vs cubic interpolation, or rounding differences");
}

/// Compare TRC handling across different profile types
#[test]
fn compare_trc_across_profiles() {
    eprintln!("\n=== TRC Handling Across Profile Types ===\n");

    let profiles_dir = testdata_dir().join("profiles");

    // Test profiles with different TRC characteristics
    let test_profiles = [
        ("skcms/misc/SM245B.icc", "Monitor (large TRC table)"),
        ("skcms/misc/BenQ_GL2450.icc", "Monitor (large TRC table)"),
        ("skcms/misc/AdobeRGB.icc", "Standard (parametric TRC)"),
        ("skcms/mobile/sRGB_parametric.icc", "sRGB (parametric)"),
        ("skcms/mobile/sRGB_LUT.icc", "sRGB (LUT-based)"),
    ];

    for (profile_rel, description) in &test_profiles {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            eprintln!("SKIP: {} (not found)", profile_rel);
            continue;
        }

        let data = match std::fs::read(&profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let moxcms_profile = match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(p) => p,
            Err(_) => {
                eprintln!("SKIP: {} (moxcms parse failed)", profile_rel);
                continue;
            }
        };

        let qcms_profile = match qcms::Profile::new_from_slice(&data, false) {
            Some(p) => p,
            None => continue,
        };

        let skcms_profile = match skcms_sys::parse_icc_profile(&data) {
            Some(p) => p,
            None => continue,
        };

        // Create transforms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = match moxcms_profile.create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        ) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let qcms_srgb = qcms::Profile::new_sRGB();
        let qcms_transform = match qcms::Transform::new(
            &qcms_profile,
            &qcms_srgb,
            qcms::DataType::RGB8,
            qcms::Intent::Perceptual,
        ) {
            Some(t) => t,
            None => continue,
        };

        let skcms_srgb = skcms_sys::srgb_profile();

        // Test at key points
        let test_values = [64u8, 128, 192, 255];
        let mut diffs = Vec::new();

        for &v in &test_values {
            let color = [v, v, v];

            let mut qcms_out = color.to_vec();
            qcms_transform.apply(&mut qcms_out);

            let mut moxcms_out = [0u8; 3];
            moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

            let mut skcms_out = [0u8; 3];
            skcms_sys::transform(
                &color,
                skcms_PixelFormat::RGB_888,
                skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut skcms_out,
                skcms_PixelFormat::RGB_888,
                skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );

            let browser_avg = ((qcms_out[0] as i32 + skcms_out[0] as i32) / 2) as i32;
            let diff = moxcms_out[0] as i32 - browser_avg;
            diffs.push(diff);
        }

        let max_diff = diffs.iter().map(|d| d.abs()).max().unwrap();
        let diff_str: String = diffs.iter().map(|d| format!("{:+}", d)).collect::<Vec<_>>().join(", ");

        eprintln!(
            "{}: max={:2}, pattern=[{}]",
            description, max_diff, diff_str
        );
    }

    eprintln!("\nNote: Pattern [+,+,+,+] = consistently brighter");
    eprintln!("      Pattern [0,0,0,0] = exact match");
    eprintln!("      Pattern [-,-,-,-] = consistently darker");
}

/// Test identity transform to isolate TRC issues
#[test]
fn test_identity_trc_isolation() {
    eprintln!("\n=== Identity Transform TRC Isolation ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    // Create identity transform (SM245B -> SM245B)
    let identity_transform = match moxcms_profile.create_transform_8bit(
        moxcms::Layout::Rgb,
        &moxcms_profile,
        moxcms::Layout::Rgb,
        moxcms::TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Identity transform failed: {:?}", e);
            return;
        }
    };

    eprintln!("Identity transform (SM245B -> SM245B):");
    eprintln!("Input | Output | Diff");
    eprintln!("------|--------|-----");

    for v in (0..=255).step_by(16) {
        let color = [v as u8, v as u8, v as u8];
        let mut out = [0u8; 3];
        identity_transform.transform(&color, &mut out).unwrap();

        let diff = out[0] as i32 - v as i32;
        eprintln!("  {:3} |  {:3}   | {:+3}", v, out[0], diff);
    }

    eprintln!("\nNote: Identity transform should output = input (diff = 0)");
    eprintln!("Any deviation indicates TRC handling or precision issues.");
}
