//! Browser CMS (skcms/qcms) Parity Tests
//!
//! Since skcms (Chrome/Skia) and qcms (Firefox) are used in browsers for billions of
//! color transforms daily, their behavior should be treated as authoritative reference
//! alongside the ICC spec. Any differences between browser CMS implementations and
//! lcms2/moxcms are flagged here for investigation.
//!
//! Key insight: Browser CMS may have made deliberate choices for performance or
//! compatibility that differ from strict ICC interpretation.

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

/// Test sRGB identity transform consistency across all CMS
#[test]
fn test_srgb_identity_browser_parity() {
    eprintln!("\n=== Browser CMS sRGB Identity Parity ===");

    // Test colors covering gamut edges and common values
    let test_colors: Vec<([u8; 3], &str)> = vec![
        ([0, 0, 0], "black"),
        ([255, 255, 255], "white"),
        ([128, 128, 128], "mid-gray"),
        ([255, 0, 0], "pure red"),
        ([0, 255, 0], "pure green"),
        ([0, 0, 255], "pure blue"),
        ([255, 255, 0], "yellow"),
        ([0, 255, 255], "cyan"),
        ([255, 0, 255], "magenta"),
        ([1, 1, 1], "near-black"),
        ([254, 254, 254], "near-white"),
        ([18, 18, 18], "video black"),
        ([235, 235, 235], "video white"),
    ];

    // Setup all CMS
    let qcms_srgb = qcms::Profile::new_sRGB();
    let qcms_transform = qcms::Transform::new(
        &qcms_srgb,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )
    .unwrap();

    let moxcms_srgb = moxcms::ColorProfile::new_srgb();
    let moxcms_transform = moxcms_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let lcms2_srgb = lcms2::Profile::new_srgb();
    let lcms2_transform = lcms2::Transform::new(
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .unwrap();

    let skcms_srgb = skcms_sys::srgb_profile();

    let mut differences_found = Vec::new();

    for (color, name) in &test_colors {
        // qcms (in-place)
        let mut qcms_out = color.to_vec();
        qcms_transform.apply(&mut qcms_out);

        // moxcms
        let mut moxcms_out = [0u8; 3];
        moxcms_transform.transform(color, &mut moxcms_out).unwrap();

        // lcms2
        let mut lcms2_out = [0u8; 3];
        lcms2_transform.transform_pixels(color, &mut lcms2_out);

        // skcms
        let mut skcms_out = [0u8; 3];
        skcms_sys::transform(
            color,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            skcms_srgb,
            &mut skcms_out,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );

        // Check for browser CMS differences
        let qcms_out_arr = [qcms_out[0], qcms_out[1], qcms_out[2]];

        // Find max difference between any pair
        let max_diff = |a: &[u8; 3], b: &[u8; 3]| -> i32 {
            (0..3).map(|i| (a[i] as i32 - b[i] as i32).abs()).max().unwrap()
        };

        let qcms_vs_skcms = max_diff(&qcms_out_arr, &skcms_out);
        let qcms_vs_lcms2 = max_diff(&qcms_out_arr, &lcms2_out);
        let skcms_vs_lcms2 = max_diff(&skcms_out, &lcms2_out);
        let moxcms_vs_lcms2 = max_diff(&moxcms_out, &lcms2_out);
        let qcms_vs_moxcms = max_diff(&qcms_out_arr, &moxcms_out);
        let skcms_vs_moxcms = max_diff(&skcms_out, &moxcms_out);

        // Flag any differences between browsers themselves
        if qcms_vs_skcms > 0 {
            differences_found.push(format!(
                "BROWSER DIFF: {} {:?} qcms={:?} vs skcms={:?} (diff={})",
                name, color, qcms_out_arr, skcms_out, qcms_vs_skcms
            ));
        }

        // Flag differences between browsers and reference CMS
        if qcms_vs_lcms2 > 0 || skcms_vs_lcms2 > 0 {
            if qcms_vs_lcms2 == skcms_vs_lcms2 && qcms_vs_skcms == 0 {
                // Both browsers agree but differ from lcms2 - browser consensus
                differences_found.push(format!(
                    "BROWSER CONSENSUS: {} {:?} browsers={:?} vs lcms2={:?} (diff={})",
                    name, color, qcms_out_arr, lcms2_out, qcms_vs_lcms2
                ));
            }
        }

        // Check moxcms alignment
        if moxcms_vs_lcms2 > 0 {
            // moxcms differs from lcms2
            if qcms_vs_moxcms == 0 || skcms_vs_moxcms == 0 {
                // moxcms agrees with a browser
                let browser = if qcms_vs_moxcms == 0 { "qcms" } else { "skcms" };
                differences_found.push(format!(
                    "MOXCMS ALIGNS WITH {}: {} {:?} moxcms={:?} lcms2={:?}",
                    browser, name, color, moxcms_out, lcms2_out
                ));
            }
        }
    }

    if differences_found.is_empty() {
        eprintln!("  All CMS produce identical sRGB identity output");
    } else {
        eprintln!("  Differences found:");
        for diff in &differences_found {
            eprintln!("    {}", diff);
        }
    }

    // sRGB identity should be identical across all CMS
    assert!(
        differences_found.is_empty(),
        "sRGB identity should produce identical results across all CMS"
    );
}

/// Test rendering intent consistency between browser CMS
#[test]
fn test_rendering_intent_browser_parity() {
    eprintln!("\n=== Browser CMS Rendering Intent Parity ===");

    let intents = [
        (qcms::Intent::Perceptual, lcms2::Intent::Perceptual, moxcms::RenderingIntent::Perceptual, "Perceptual"),
        (qcms::Intent::RelativeColorimetric, lcms2::Intent::RelativeColorimetric, moxcms::RenderingIntent::RelativeColorimetric, "RelativeColorimetric"),
        (qcms::Intent::Saturation, lcms2::Intent::Saturation, moxcms::RenderingIntent::Saturation, "Saturation"),
        (qcms::Intent::AbsoluteColorimetric, lcms2::Intent::AbsoluteColorimetric, moxcms::RenderingIntent::AbsoluteColorimetric, "AbsoluteColorimetric"),
    ];

    let test_colors: Vec<[u8; 3]> = vec![
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [128, 128, 128],
    ];

    let mut intent_differences = Vec::new();

    for (qcms_intent, lcms2_intent, moxcms_intent, intent_name) in &intents {
        let qcms_srgb = qcms::Profile::new_sRGB();
        let qcms_transform = qcms::Transform::new(
            &qcms_srgb,
            &qcms_srgb,
            qcms::DataType::RGB8,
            *qcms_intent,
        )
        .unwrap();

        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = moxcms_srgb
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &moxcms_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions {
                    rendering_intent: *moxcms_intent,
                    ..Default::default()
                },
            )
            .unwrap();

        let lcms2_srgb = lcms2::Profile::new_srgb();
        let lcms2_transform = lcms2::Transform::new(
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_8,
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_8,
            *lcms2_intent,
        )
        .unwrap();

        for color in &test_colors {
            let mut qcms_out = color.to_vec();
            qcms_transform.apply(&mut qcms_out);

            let mut moxcms_out = [0u8; 3];
            moxcms_transform.transform(color, &mut moxcms_out).unwrap();

            let mut lcms2_out = [0u8; 3];
            lcms2_transform.transform_pixels(color, &mut lcms2_out);

            let qcms_out_arr = [qcms_out[0], qcms_out[1], qcms_out[2]];

            let qcms_vs_lcms2 = (0..3)
                .map(|i| (qcms_out_arr[i] as i32 - lcms2_out[i] as i32).abs())
                .max()
                .unwrap();

            let moxcms_vs_lcms2 = (0..3)
                .map(|i| (moxcms_out[i] as i32 - lcms2_out[i] as i32).abs())
                .max()
                .unwrap();

            if qcms_vs_lcms2 > 0 || moxcms_vs_lcms2 > 0 {
                intent_differences.push(format!(
                    "{} intent {:?}: qcms={:?}(diff={}) moxcms={:?}(diff={}) lcms2={:?}",
                    intent_name, color, qcms_out_arr, qcms_vs_lcms2,
                    moxcms_out, moxcms_vs_lcms2, lcms2_out
                ));
            }
        }
    }

    if intent_differences.is_empty() {
        eprintln!("  All intents produce identical results across CMS");
    } else {
        eprintln!("  Intent differences found:");
        for diff in intent_differences.iter().take(10) {
            eprintln!("    {}", diff);
        }
        if intent_differences.len() > 10 {
            eprintln!("    ... and {} more", intent_differences.len() - 10);
        }
    }
}

/// Test gamma/TRC curve evaluation differences
#[test]
fn test_trc_curve_browser_parity() {
    eprintln!("\n=== Browser CMS TRC Curve Parity ===");

    // Test values at key points in the TRC curve
    let test_values: Vec<u8> = (0..=255).step_by(17).collect();

    let qcms_srgb = qcms::Profile::new_sRGB();
    let moxcms_srgb = moxcms::ColorProfile::new_srgb();
    let lcms2_srgb = lcms2::Profile::new_srgb();
    let skcms_srgb = skcms_sys::srgb_profile();

    let qcms_transform = qcms::Transform::new(
        &qcms_srgb,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )
    .unwrap();

    let moxcms_transform = moxcms_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    let lcms2_transform = lcms2::Transform::new(
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .unwrap();

    let mut curve_diffs = Vec::new();

    for &v in &test_values {
        let color = [v, v, v]; // Test on neutral axis

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
            skcms_srgb,
            &mut skcms_out,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );

        if qcms_out[0] != lcms2_out[0] || skcms_out[0] != lcms2_out[0] || moxcms_out[0] != lcms2_out[0] {
            curve_diffs.push(format!(
                "  Input={}: qcms={} skcms={} moxcms={} lcms2={}",
                v, qcms_out[0], skcms_out[0], moxcms_out[0], lcms2_out[0]
            ));
        }
    }

    if curve_diffs.is_empty() {
        eprintln!("  TRC curve evaluation is identical across all CMS");
    } else {
        eprintln!("  TRC curve differences at neutral axis:");
        for diff in &curve_diffs {
            eprintln!("{}", diff);
        }
    }

    // For sRGB identity, TRC should be identical
    assert!(curve_diffs.is_empty(), "TRC curves should match for sRGB identity");
}

/// Test external profile transform parity between browsers
#[test]
fn test_external_profile_browser_parity() {
    eprintln!("\n=== Browser CMS External Profile Parity ===");

    let profiles_dir = testdata_dir().join("profiles");

    // Test with real-world profiles that all browsers can parse
    let test_profiles = [
        "qcms/sRGB_lcms.icc",
        "skcms/misc/AdobeRGB.icc",
        "skcms/mobile/Display_P3_parametric.icc",
    ];

    let test_color = [255u8, 128, 64];
    let srgb_skcms = skcms_sys::srgb_profile();

    for profile_rel in &test_profiles {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            eprintln!("  SKIP: {} (not found)", profile_rel);
            continue;
        }

        let data = std::fs::read(&profile_path).unwrap();
        let name = profile_path.file_name().unwrap().to_string_lossy();

        // Try to load with all CMS
        let qcms_profile = match qcms::Profile::new_from_slice(&data, false) {
            Some(p) => p,
            None => {
                eprintln!("  SKIP: {} (qcms parse failed)", name);
                continue;
            }
        };

        let moxcms_profile = match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(p) => p,
            Err(_) => {
                eprintln!("  SKIP: {} (moxcms parse failed)", name);
                continue;
            }
        };

        let lcms2_profile = match lcms2::Profile::new_icc(&data) {
            Ok(p) => p,
            Err(_) => {
                eprintln!("  SKIP: {} (lcms2 parse failed)", name);
                continue;
            }
        };

        let skcms_profile = match skcms_sys::parse_icc_profile(&data) {
            Some(p) => p,
            None => {
                eprintln!("  SKIP: {} (skcms parse failed)", name);
                continue;
            }
        };

        // Create transforms: profile -> sRGB
        let qcms_srgb = qcms::Profile::new_sRGB();
        let qcms_transform = match qcms::Transform::new(
            &qcms_profile,
            &qcms_srgb,
            qcms::DataType::RGB8,
            qcms::Intent::Perceptual,
        ) {
            Some(t) => t,
            None => {
                eprintln!("  SKIP: {} (qcms transform failed)", name);
                continue;
            }
        };

        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = match moxcms_profile.create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        ) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("  SKIP: {} (moxcms transform failed)", name);
                continue;
            }
        };

        let lcms2_srgb = lcms2::Profile::new_srgb();
        let lcms2_transform = match lcms2::Transform::new(
            &lcms2_profile,
            lcms2::PixelFormat::RGB_8,
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_8,
            lcms2::Intent::Perceptual,
        ) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("  SKIP: {} (lcms2 transform failed)", name);
                continue;
            }
        };

        // Transform
        let mut qcms_out = test_color.to_vec();
        qcms_transform.apply(&mut qcms_out);

        let mut moxcms_out = [0u8; 3];
        moxcms_transform.transform(&test_color, &mut moxcms_out).unwrap();

        let mut lcms2_out = [0u8; 3];
        lcms2_transform.transform_pixels(&test_color, &mut lcms2_out);

        let mut skcms_out = [0u8; 3];
        skcms_sys::transform(
            &test_color,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut skcms_out,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            srgb_skcms,
            1,
        );

        // Compare browsers
        let qcms_arr = [qcms_out[0], qcms_out[1], qcms_out[2]];

        let qcms_vs_skcms = (0..3)
            .map(|i| (qcms_arr[i] as i32 - skcms_out[i] as i32).abs())
            .max()
            .unwrap();

        let browser_avg = [
            ((qcms_arr[0] as u16 + skcms_out[0] as u16) / 2) as u8,
            ((qcms_arr[1] as u16 + skcms_out[1] as u16) / 2) as u8,
            ((qcms_arr[2] as u16 + skcms_out[2] as u16) / 2) as u8,
        ];

        let moxcms_vs_browsers = (0..3)
            .map(|i| (moxcms_out[i] as i32 - browser_avg[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!(
            "  {}: qcms={:?} skcms={:?} moxcms={:?} lcms2={:?}",
            name, qcms_arr, skcms_out, moxcms_out, lcms2_out
        );
        eprintln!(
            "    Browser diff: {} | moxcms vs browser avg: {}",
            qcms_vs_skcms, moxcms_vs_browsers
        );

        // Flag significant browser disagreement
        if qcms_vs_skcms > 2 {
            eprintln!("    WARNING: Browsers disagree by {} on {}", qcms_vs_skcms, name);
        }

        // Flag if moxcms deviates significantly from browser consensus
        if moxcms_vs_browsers > 2 && qcms_vs_skcms <= 1 {
            eprintln!(
                "    REVIEW: moxcms differs from browser consensus by {} on {}",
                moxcms_vs_browsers, name
            );
        }
    }
}

/// Test for known browser-specific behaviors
#[test]
fn test_browser_specific_behaviors() {
    eprintln!("\n=== Known Browser CMS Behaviors ===");

    // Document known differences between browser CMS implementations
    // These are not failures - they're documentation of browser choices

    eprintln!("  Known qcms behaviors:");
    eprintln!("    - Uses fixed-point math for transforms (faster, slight precision loss)");
    eprintln!("    - In-place transform API only");
    eprintln!("    - No support for Gray8 transforms");

    eprintln!("\n  Known skcms behaviors:");
    eprintln!("    - SIMD-optimized with AVX2/AVX-512 codepaths");
    eprintln!("    - Separate src/dst buffer API");
    eprintln!("    - Built for Skia/Chrome's specific needs");

    eprintln!("\n  Implications for oxcms:");
    eprintln!("    - When browsers agree but differ from lcms2, follow browser consensus");
    eprintln!("    - Performance should match or exceed browser CMS");
    eprintln!("    - Parsing should be at least as permissive as browsers");
}
