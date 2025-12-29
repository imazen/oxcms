//! Reproducible test case: CMYK pure yellow axis produces incorrect green channel
//!
//! When transforming CMYK [0,0,Y,0] (pure yellow) to sRGB, the green channel
//! is 2-8 values too low compared to lcms2 and skcms reference implementations.
//!
//! ## Expected values (from lcms2, verified against skcms)
//!
//! | CMYK Y | Expected G | moxcms G | Δ |
//! |--------|------------|----------|---|
//! | 64     | 251        | 248      | 3 |
//! | 128    | 247        | 242      | 5 |
//! | 192    | 244        | 237      | 7 |
//! | 224    | 243        | 235      | 8 |
//!
//! Profile: us_swop_coated.icc (US Web Coated SWOP v2)

use moxcms::{ColorProfile, Layout, TransformOptions};
use std::path::PathBuf;

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Reference values from lcms2 for pure yellow axis [0,0,Y,0] -> RGB
/// Format: (Y_value, expected_R, expected_G, expected_B)
const LCMS2_REFERENCE: &[(u8, u8, u8, u8)] = &[
    (0,   255, 255, 255),
    (32,  255, 253, 228),
    (64,  255, 251, 204),
    (96,  255, 249, 178),
    (128, 255, 247, 153),
    (160, 255, 246, 127),
    (192, 255, 244, 94),
    (224, 255, 243, 50),
    (255, 255, 242, 0),
];

#[test]
fn test_pure_yellow_green_channel() {
    let profile_path = assets_dir().join("us_swop_coated.icc");
    let profile_data = std::fs::read(&profile_path)
        .expect("us_swop_coated.icc required in assets/");

    let cmyk_profile = ColorProfile::new_from_slice(&profile_data)
        .expect("Failed to parse CMYK profile");
    let srgb = ColorProfile::new_srgb();

    let transform = cmyk_profile
        .create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgb, TransformOptions::default())
        .expect("Failed to create transform");

    println!("\n=== Pure Yellow Axis Test ===\n");
    println!("{:<12} {:>10} {:>10} {:>6}", "CMYK[Y]", "expected", "actual", "Δ");
    println!("{}", "-".repeat(42));

    let mut max_diff = 0i16;
    let mut failures = Vec::new();

    for &(y, exp_r, exp_g, exp_b) in LCMS2_REFERENCE {
        let cmyk: [u8; 4] = [0, 0, y, 0];
        let mut rgb = [0u8; 3];
        transform.transform(&cmyk, &mut rgb).unwrap();

        let diff_g = (exp_g as i16 - rgb[1] as i16).abs();
        if diff_g > max_diff {
            max_diff = diff_g;
        }

        println!(
            "[0,0,{:3},0] {:>3},{:>3},{:>3}  {:>3},{:>3},{:>3} {:>6}",
            y, exp_r, exp_g, exp_b, rgb[0], rgb[1], rgb[2], diff_g
        );

        if diff_g > 1 {
            failures.push((y, exp_g, rgb[1], diff_g));
        }
    }

    println!("\nMax green channel difference: {}", max_diff);

    if !failures.is_empty() {
        println!("\nFailing cases (Δ > 1):");
        for (y, expected, actual, diff) in &failures {
            println!("  Y={}: expected G={}, got G={}, Δ={}", y, expected, actual, diff);
        }
    }

    assert!(
        max_diff <= 1,
        "Green channel differs by {} from lcms2 reference (expected ≤1). \
         This affects pure yellow CMYK values [0,0,Y,0].",
        max_diff
    );
}

/// Test that interpolation method doesn't affect this bug
#[test]
#[cfg(feature = "options")]
fn test_yellow_bug_interpolation_independent() {
    use moxcms::InterpolationMethod;

    let profile_path = assets_dir().join("us_swop_coated.icc");
    let profile_data = std::fs::read(&profile_path).expect("us_swop_coated.icc required");

    let cmyk_profile = ColorProfile::new_from_slice(&profile_data).unwrap();
    let srgb = ColorProfile::new_srgb();

    let cmyk: [u8; 4] = [0, 0, 192, 0]; // 75% yellow

    // Test with default (trilinear)
    let default_opts = TransformOptions::default();
    let transform_default = cmyk_profile
        .create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgb, default_opts)
        .unwrap();
    let mut rgb_default = [0u8; 3];
    transform_default.transform(&cmyk, &mut rgb_default).unwrap();

    // Test with tetrahedral
    let tet_opts = TransformOptions {
        interpolation_method: InterpolationMethod::Tetrahedral,
        ..TransformOptions::default()
    };
    let transform_tet = cmyk_profile
        .create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgb, tet_opts)
        .unwrap();
    let mut rgb_tet = [0u8; 3];
    transform_tet.transform(&cmyk, &mut rgb_tet).unwrap();

    println!("\n=== Interpolation Method Comparison ===\n");
    println!("CMYK [0,0,192,0] (75% yellow):");
    println!("  Trilinear:   RGB {:?}", rgb_default);
    println!("  Tetrahedral: RGB {:?}", rgb_tet);
    println!("  lcms2 ref:   RGB [255, 244, 94]");

    // Both should be the same (bug is not in interpolation)
    assert_eq!(
        rgb_default, rgb_tet,
        "Trilinear and tetrahedral should produce same result for pure yellow"
    );

    // Both should match lcms2 (this will fail, demonstrating the bug)
    let expected_g = 244u8;
    let diff = (expected_g as i16 - rgb_default[1] as i16).abs();
    assert!(
        diff <= 1,
        "Green channel {} differs from lcms2 reference {} by {} (both interpolation methods)",
        rgb_default[1], expected_g, diff
    );
}
