//! CMYK Near-White Color Tests
//!
//! ## Findings (2026-01-02)
//!
//! moxcms matches lcms2 (industry standard) for CMYK paper white:
//!
//! | CMYK Input | moxcms Output | lcms2 Output | skcms Output |
//! |------------|---------------|--------------|--------------|
//! | [0,0,0,0]  | [255,255,255] | [255,255,255]| [252,254,255]|
//!
//! **skcms produces a slightly different result** than lcms2/moxcms.
//! This is not a bug in moxcms - it's a difference in how skcms handles CMYK.
//!
//! ## Impact on libjxl vs jxl-rs
//!
//! Since libjxl uses skcms and jxl-rs uses moxcms, CMYK images will have
//! small differences (~3 units) in near-white areas. This explains the
//! jxl-rs vs libjxl parity gap for cmyk_layers.jxl.
//!
//! Profile: cmyk_layers.icc (from JPEG XL cmyk_layers.jxl conformance test)

use moxcms::{ColorProfile, Layout, TransformOptions};
use std::path::PathBuf;

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Test near-white CMYK values with 8-bit transform
#[test]
fn test_nearwhite_cmyk_8bit() {
    let profile_path = assets_dir().join("cmyk_layers.icc");
    let profile_data = std::fs::read(&profile_path).expect("cmyk_layers.icc required in assets/");

    let cmyk_profile =
        ColorProfile::new_from_slice(&profile_data).expect("Failed to parse CMYK profile");
    let srgb = ColorProfile::new_srgb();

    let transform = cmyk_profile
        .create_transform_8bit(
            Layout::Rgba,
            &srgb,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .expect("Failed to create transform");

    println!("\n=== Near-White CMYK Test (8-bit) ===\n");
    println!("{:<16} {:>10} {:>10}", "CMYK", "RGB output", "white?");
    println!("{}", "-".repeat(40));

    // Test various near-white CMYK values
    // Format: [C, M, Y, K] in 0-255 scale
    let test_cases: &[[u8; 4]] = &[
        [0, 0, 0, 0],  // Pure white - should be [255,255,255]
        [1, 0, 0, 0],  // Tiny cyan
        [0, 1, 0, 0],  // Tiny magenta
        [0, 0, 1, 0],  // Tiny yellow
        [1, 1, 0, 0],  // Tiny blue (C+M)
        [1, 0, 1, 0],  // Tiny green (C+Y)
        [0, 1, 1, 0],  // Tiny red (M+Y)
        [2, 1, 0, 0],  // Slight blue - should NOT be pure white
        [3, 1, 0, 0],  // More blue
        [5, 2, 0, 0],  // ~2% C, 0.8% M
        [10, 5, 2, 0], // Low saturation
    ];

    let mut clipped_count = 0;

    for cmyk in test_cases {
        let mut rgb = [0u8; 3];
        transform.transform(cmyk, &mut rgb).unwrap();

        let is_clipped = rgb == [255, 255, 255] && *cmyk != [0, 0, 0, 0];
        if is_clipped {
            clipped_count += 1;
        }

        println!(
            "[{:3},{:3},{:3},{:3}] [{:3},{:3},{:3}]  {}",
            cmyk[0],
            cmyk[1],
            cmyk[2],
            cmyk[3],
            rgb[0],
            rgb[1],
            rgb[2],
            if is_clipped { "<-- CLIPPED!" } else { "" }
        );
    }

    println!(
        "\nClipped to pure white: {}/{}",
        clipped_count,
        test_cases.len() - 1
    );

    // Non-zero CMYK should never produce pure white
    assert_eq!(
        clipped_count, 0,
        "{} near-white CMYK values were incorrectly clipped to [255,255,255]. \
         This indicates a clipping bug - near-white colors should preserve subtle tints.",
        clipped_count
    );
}

/// Test near-white CMYK values with f32 transform
#[test]
fn test_nearwhite_cmyk_f32() {
    let profile_path = assets_dir().join("cmyk_layers.icc");
    let profile_data = std::fs::read(&profile_path).expect("cmyk_layers.icc required in assets/");

    let cmyk_profile =
        ColorProfile::new_from_slice(&profile_data).expect("Failed to parse CMYK profile");
    let srgb = ColorProfile::new_srgb();

    let transform = cmyk_profile
        .create_transform_f32(
            Layout::Rgba,
            &srgb,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .expect("Failed to create transform");

    println!("\n=== Near-White CMYK Test (f32) ===\n");
    println!("{:<20} {:>16}", "CMYK", "RGB output");
    println!("{}", "-".repeat(40));

    // Test with f32 values - more precision for near-white
    // These are ICC convention: 0.0 = no ink, 1.0 = full ink
    let test_cases: &[[f32; 4]] = &[
        [0.0, 0.0, 0.0, 0.0],    // Pure white
        [0.01, 0.0, 0.0, 0.0],   // 1% cyan
        [0.0, 0.01, 0.0, 0.0],   // 1% magenta
        [0.0, 0.0, 0.01, 0.0],   // 1% yellow
        [0.01, 0.005, 0.0, 0.0], // ~1% C, 0.5% M -> should be slight blue tint
        [0.02, 0.01, 0.0, 0.0],  // ~2% C, 1% M -> more blue
        [0.05, 0.02, 0.01, 0.0], // Mixed low ink
    ];

    let mut issues = Vec::new();

    for cmyk in test_cases {
        let mut rgb = [0.0f32; 3];
        transform.transform(cmyk, &mut rgb).unwrap();

        // Convert to 0-255 for display
        let rgb_u8 = [
            (rgb[0] * 255.0).round().clamp(0.0, 255.0) as u8,
            (rgb[1] * 255.0).round().clamp(0.0, 255.0) as u8,
            (rgb[2] * 255.0).round().clamp(0.0, 255.0) as u8,
        ];

        let is_clipped = rgb_u8 == [255, 255, 255] && *cmyk != [0.0, 0.0, 0.0, 0.0];
        if is_clipped {
            issues.push(*cmyk);
        }

        println!(
            "[{:.3},{:.3},{:.3},{:.3}] [{:.3},{:.3},{:.3}] -> [{:3},{:3},{:3}] {}",
            cmyk[0],
            cmyk[1],
            cmyk[2],
            cmyk[3],
            rgb[0],
            rgb[1],
            rgb[2],
            rgb_u8[0],
            rgb_u8[1],
            rgb_u8[2],
            if is_clipped { "<-- CLIPPED!" } else { "" }
        );
    }

    if !issues.is_empty() {
        println!("\n{} values clipped to white:", issues.len());
        for cmyk in &issues {
            println!(
                "  [{:.3},{:.3},{:.3},{:.3}]",
                cmyk[0], cmyk[1], cmyk[2], cmyk[3]
            );
        }
    }

    assert!(
        issues.is_empty(),
        "Near-white CMYK values should NOT be clipped to pure white. \
         {} test cases produced [255,255,255] when they should have subtle color tints.",
        issues.len()
    );
}

/// Compare with us_swop_coated.icc to see if issue is profile-specific
#[test]
fn test_nearwhite_compare_profiles() {
    let cmyk_layers_path = assets_dir().join("cmyk_layers.icc");
    let us_swop_path = assets_dir().join("us_swop_coated.icc");

    let cmyk_layers_data = std::fs::read(&cmyk_layers_path).ok();
    let us_swop_data = std::fs::read(&us_swop_path).ok();

    let srgb = ColorProfile::new_srgb();
    let test_cmyk: [u8; 4] = [5, 2, 0, 0]; // ~2% C, 0.8% M

    println!(
        "\n=== Profile Comparison for CMYK [{},{},{},{}] ===\n",
        test_cmyk[0], test_cmyk[1], test_cmyk[2], test_cmyk[3]
    );

    if let Some(data) = cmyk_layers_data {
        let profile = ColorProfile::new_from_slice(&data).expect("parse cmyk_layers.icc");
        let transform = profile
            .create_transform_8bit(
                Layout::Rgba,
                &srgb,
                Layout::Rgb,
                TransformOptions::default(),
            )
            .expect("create transform");
        let mut rgb = [0u8; 3];
        transform.transform(&test_cmyk, &mut rgb).unwrap();
        println!(
            "cmyk_layers.icc:   RGB [{:3},{:3},{:3}]",
            rgb[0], rgb[1], rgb[2]
        );
    }

    if let Some(data) = us_swop_data {
        let profile = ColorProfile::new_from_slice(&data).expect("parse us_swop_coated.icc");
        let transform = profile
            .create_transform_8bit(
                Layout::Rgba,
                &srgb,
                Layout::Rgb,
                TransformOptions::default(),
            )
            .expect("create transform");
        let mut rgb = [0u8; 3];
        transform.transform(&test_cmyk, &mut rgb).unwrap();
        println!(
            "us_swop_coated.icc: RGB [{:3},{:3},{:3}]",
            rgb[0], rgb[1], rgb[2]
        );
    }

    // Both should produce similar near-white values, not pure white
    println!("\nExpected: RGB close to [252-254, 254-255, 255] (subtle cyan tint)");
    println!("Bug:      RGB [255, 255, 255] (incorrectly clipped to white)");
}

/// Document paper white behavior - moxcms matches lcms2, differs from skcms
#[test]
fn test_paper_white_behavior() {
    let profile_path = assets_dir().join("cmyk_layers.icc");
    let profile_data = std::fs::read(&profile_path).expect("cmyk_layers.icc required in assets/");

    let cmyk_profile =
        ColorProfile::new_from_slice(&profile_data).expect("Failed to parse CMYK profile");
    let srgb = ColorProfile::new_srgb();

    let transform = cmyk_profile
        .create_transform_8bit(
            Layout::Rgba,
            &srgb,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .expect("Failed to create transform");

    // CMYK [0,0,0,0] = no ink = paper white
    let paper_white_cmyk: [u8; 4] = [0, 0, 0, 0];
    let mut rgb = [0u8; 3];
    transform.transform(&paper_white_cmyk, &mut rgb).unwrap();

    println!("\n=== Paper White Behavior Test ===\n");
    println!("Input:    CMYK [0, 0, 0, 0] (no ink = paper white)");
    println!("moxcms:   RGB [{}, {}, {}]", rgb[0], rgb[1], rgb[2]);
    println!("lcms2:    RGB [255, 255, 255] (matches moxcms)");
    println!("skcms:    RGB [252, 254, 255] (different!)");

    // moxcms matches lcms2 (industry standard)
    // skcms produces slightly different output for CMYK paper white
    let expected_r = 255u8; // lcms2 value
    let expected_g = 255u8;
    let expected_b = 255u8;

    let r_diff = (rgb[0] as i16 - expected_r as i16).abs();
    let g_diff = (rgb[1] as i16 - expected_g as i16).abs();
    let b_diff = (rgb[2] as i16 - expected_b as i16).abs();
    let max_diff = r_diff.max(g_diff).max(b_diff);

    println!("\nmoxcms vs lcms2 difference: {}", max_diff);
    println!("(skcms differs from lcms2/moxcms by ~3 units)");

    assert!(
        max_diff <= 1,
        "moxcms should match lcms2 for paper white, got difference of {}",
        max_diff
    );
}
