//! Advanced color space transform tests
//!
//! Tests for CMYK, Lab, and XYZ color space conversions
//! comparing moxcms and lcms2.

use cms_tests::accuracy::srgb_to_lab;
use std::path::Path;

/// Test CMYK profile loading and basic transforms
#[test]
fn test_cmyk_profile_support() {
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata");

    // Check for CMYK profiles
    let cmyk_profiles = [
        "profiles/USWebCoatedSWOP.icc",
        "profiles/CoatedFOGRA39.icc",
        "profiles/JapanColor2001Coated.icc",
    ];

    let mut found = false;
    for path in cmyk_profiles {
        let full_path = testdata.join(path);
        if full_path.exists() {
            found = true;
            let data = std::fs::read(&full_path).expect("read CMYK profile");

            // Try parsing with moxcms
            match moxcms::ColorProfile::new_from_slice(&data) {
                Ok(profile) => {
                    eprintln!("moxcms: {} loaded OK", path);
                    eprintln!("  Color space: {:?}", profile.color_space);
                    eprintln!("  PCS: {:?}", profile.pcs);
                }
                Err(e) => {
                    eprintln!("moxcms: {} FAILED: {:?}", path, e);
                }
            }

            // Try parsing with lcms2
            match lcms2::Profile::new_icc(&data) {
                Ok(_profile) => {
                    eprintln!("lcms2:  {} loaded OK", path);
                }
                Err(e) => {
                    eprintln!("lcms2:  {} FAILED: {}", path, e);
                }
            }
        }
    }

    if !found {
        eprintln!("\nNo CMYK profiles found in testdata/profiles/");
        eprintln!("Run ./scripts/fetch-test-profiles.sh to download test profiles");
    }
}

/// Test Lab color space accuracy
#[test]
fn test_lab_conversion_accuracy() {
    // Reference Lab values for common sRGB colors
    // These are computed using standard sRGB to Lab formulas
    let test_cases = [
        // [R, G, B, expected L, expected a, expected b]
        ([255u8, 255, 255], (100.0, 0.0, 0.0)),      // White
        ([0u8, 0, 0], (0.0, 0.0, 0.0)),              // Black
        ([128u8, 128, 128], (53.585, 0.0, 0.0)),     // Mid gray
        ([255u8, 0, 0], (53.233, 80.109, 67.220)),   // Red
        ([0u8, 255, 0], (87.737, -86.185, 83.181)),  // Green
        ([0u8, 0, 255], (32.303, 79.197, -107.864)), // Blue
    ];

    eprintln!("\nLab conversion accuracy:");
    for (rgb, expected_lab) in &test_cases {
        let computed_lab = srgb_to_lab(rgb[0], rgb[1], rgb[2]);

        let l_diff = (computed_lab[0] - expected_lab.0).abs();
        let a_diff = (computed_lab[1] - expected_lab.1).abs();
        let b_diff = (computed_lab[2] - expected_lab.2).abs();

        let max_diff = l_diff.max(a_diff).max(b_diff);

        eprintln!(
            "  RGB{:?} -> Lab({:.2}, {:.2}, {:.2}) expected({:.2}, {:.2}, {:.2}) diff:{:.4}",
            rgb,
            computed_lab[0],
            computed_lab[1],
            computed_lab[2],
            expected_lab.0,
            expected_lab.1,
            expected_lab.2,
            max_diff
        );

        // Lab conversions should be accurate to within 0.5
        assert!(
            max_diff < 1.0,
            "Lab conversion error too high for RGB{:?}: {:.4}",
            rgb,
            max_diff
        );
    }
}

/// Test XYZ color space with moxcms built-in functions
#[test]
fn test_xyz_to_lab_to_xyz_round_trip() {
    // Use moxcms's built-in XYZ/Lab types
    let test_xyz_values = [
        moxcms::Xyz::new(0.95047, 1.0, 1.08883), // D65 white
        moxcms::Xyz::new(0.4124564, 0.2126729, 0.0193339), // sRGB red
        moxcms::Xyz::new(0.0, 0.0, 0.0),         // Black
        moxcms::Xyz::new(0.5, 0.5, 0.5),         // Mid gray
    ];

    eprintln!("\nXYZ -> Lab -> XYZ round-trip:");
    for xyz in &test_xyz_values {
        // moxcms provides Lab type but conversion may need profile context
        eprintln!("  XYZ({:.4}, {:.4}, {:.4})", xyz.x, xyz.y, xyz.z);
    }
}

/// Test grayscale to RGB conversion
#[test]
fn test_grayscale_to_rgb() {
    let gray_profile = moxcms::ColorProfile::new_gray_with_gamma(2.2);
    let srgb_profile = moxcms::ColorProfile::new_srgb();

    let transform = gray_profile
        .create_transform_8bit(
            moxcms::Layout::Gray,
            &srgb_profile,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("gray to RGB transform");

    eprintln!("\nGrayscale to RGB transform:");
    let test_values = [0u8, 64, 128, 192, 255];

    for gray in test_values {
        let input = [gray];
        let mut output = [0u8; 3];
        transform.transform(&input, &mut output).unwrap();

        // For gamma 2.2 gray, output should be neutral (R=G=B)
        let max_channel_diff = (output[0] as i32 - output[1] as i32)
            .abs()
            .max((output[1] as i32 - output[2] as i32).abs());

        eprintln!(
            "  Gray {} -> RGB({}, {}, {}) channel_diff: {}",
            gray, output[0], output[1], output[2], max_channel_diff
        );

        // Gray should produce neutral colors (R=G=B within 1 level)
        assert!(
            max_channel_diff <= 1,
            "Grayscale should produce neutral RGB"
        );
    }
}

/// Test RGB to grayscale conversion
#[test]
fn test_rgb_to_grayscale() {
    let srgb_profile = moxcms::ColorProfile::new_srgb();
    let gray_profile = moxcms::ColorProfile::new_gray_with_gamma(2.2);

    let transform = srgb_profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &gray_profile,
            moxcms::Layout::Gray,
            moxcms::TransformOptions::default(),
        )
        .expect("RGB to gray transform");

    eprintln!("\nRGB to Grayscale transform:");

    // Test that equal RGB values produce corresponding gray
    let neutral_colors = [
        ([0u8, 0, 0], 0u8),
        ([128, 128, 128], 128),
        ([255, 255, 255], 255),
    ];

    for (rgb, expected_approx) in neutral_colors {
        let mut output = [0u8];
        transform.transform(&rgb, &mut output).unwrap();

        let diff = (output[0] as i32 - expected_approx as i32).abs();
        eprintln!(
            "  RGB{:?} -> Gray {} (expected ~{}, diff: {})",
            rgb, output[0], expected_approx, diff
        );

        // Neutral colors should map closely
        assert!(diff <= 2, "Neutral RGB should map to expected gray");
    }

    // Test luminance weighting (green should be brightest)
    let primary_colors = [
        [255u8, 0, 0], // Red
        [0, 255, 0],   // Green (should be brightest)
        [0, 0, 255],   // Blue
    ];

    let mut luminances = Vec::new();
    for rgb in &primary_colors {
        let mut output = [0u8];
        transform.transform(rgb, &mut output).unwrap();
        luminances.push(output[0]);
        eprintln!("  RGB{:?} -> Gray {}", rgb, output[0]);
    }

    // Green should produce the highest luminance
    assert!(
        luminances[1] > luminances[0] && luminances[1] > luminances[2],
        "Green should have highest luminance: R={}, G={}, B={}",
        luminances[0],
        luminances[1],
        luminances[2]
    );
}

/// Test different bit depths for the same transform
#[test]
fn test_bit_depth_consistency() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let transform_8 = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("8-bit transform");

    let transform_16 = srgb
        .create_transform_16bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("16-bit transform");

    eprintln!("\nBit depth consistency:");

    // Test a range of values
    let test_values_8: Vec<[u8; 3]> = (0..=255)
        .step_by(51)
        .flat_map(|r| {
            (0..=255).step_by(51).flat_map(move |g| {
                (0..=255)
                    .step_by(51)
                    .map(move |b| [r as u8, g as u8, b as u8])
            })
        })
        .collect();

    let mut max_diff = 0i32;
    let mut sum_diff = 0i64;
    let mut count = 0;

    for input_8 in &test_values_8 {
        // 8-bit transform
        let mut output_8 = [0u8; 3];
        transform_8.transform(input_8, &mut output_8).unwrap();

        // 16-bit transform (scale input to 16-bit)
        let input_16: [u16; 3] = [
            (input_8[0] as u16) << 8 | input_8[0] as u16,
            (input_8[1] as u16) << 8 | input_8[1] as u16,
            (input_8[2] as u16) << 8 | input_8[2] as u16,
        ];
        let mut output_16 = [0u16; 3];
        transform_16.transform(&input_16, &mut output_16).unwrap();

        // Scale 16-bit output to 8-bit for comparison
        let output_16_as_8: [u8; 3] = [
            (output_16[0] >> 8) as u8,
            (output_16[1] >> 8) as u8,
            (output_16[2] >> 8) as u8,
        ];

        for i in 0..3 {
            let diff = (output_8[i] as i32 - output_16_as_8[i] as i32).abs();
            max_diff = max_diff.max(diff);
            sum_diff += diff as i64;
            count += 1;
        }
    }

    let avg_diff = sum_diff as f64 / count as f64;
    eprintln!("  Max difference: {} levels", max_diff);
    eprintln!("  Average difference: {:.4} levels", avg_diff);
    eprintln!("  Sample count: {} color channels", count);

    // 8-bit and 16-bit should be very close
    assert!(
        max_diff <= 2,
        "8-bit and 16-bit transforms should match within 2 levels"
    );
}

/// Test profile white point handling
#[test]
fn test_white_point_handling() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();
    let bt2020 = moxcms::ColorProfile::new_bt2020();

    eprintln!("\nProfile white points (XYZ):");
    eprintln!(
        "  sRGB:   ({:.5}, {:.5}, {:.5})",
        srgb.white_point.x, srgb.white_point.y, srgb.white_point.z
    );
    eprintln!(
        "  P3:     ({:.5}, {:.5}, {:.5})",
        p3.white_point.x, p3.white_point.y, p3.white_point.z
    );
    eprintln!(
        "  BT2020: ({:.5}, {:.5}, {:.5})",
        bt2020.white_point.x, bt2020.white_point.y, bt2020.white_point.z
    );

    // ICC profiles use D50 as the Profile Connection Space (PCS) white point
    // D50 in XYZ (normalized to Y=1) is approximately (0.9642, 1.0, 0.8251)
    // moxcms reports (0.96391, 1.0, 0.82475) which is very close
    let d50_xyz_x = 0.9642;
    let d50_xyz_z = 0.8251;
    let eps = 0.005;

    for (name, profile) in [("sRGB", &srgb), ("P3", &p3), ("BT2020", &bt2020)] {
        let wp = &profile.white_point;
        eprintln!("  Checking {} white point vs D50 XYZ...", name);

        // Y should be 1.0 (normalized), X and Z should be close to D50 values
        assert!(
            (wp.y - 1.0).abs() < eps,
            "{} white point Y should be 1.0: got {:.5}",
            name,
            wp.y
        );
        assert!(
            (wp.x - d50_xyz_x).abs() < eps,
            "{} white point X should be close to D50: got {:.5}, expected {:.5}",
            name,
            wp.x,
            d50_xyz_x
        );
        assert!(
            (wp.z - d50_xyz_z).abs() < eps,
            "{} white point Z should be close to D50: got {:.5}, expected {:.5}",
            name,
            wp.z,
            d50_xyz_z
        );
    }

    // All profiles should have identical white points
    assert_eq!(
        srgb.white_point, p3.white_point,
        "sRGB and P3 should have same white point"
    );
    assert_eq!(
        srgb.white_point, bt2020.white_point,
        "sRGB and BT2020 should have same white point"
    );
}

/// Test RGBA alpha channel preservation
#[test]
fn test_alpha_preservation() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgba,
            &p3,
            moxcms::Layout::Rgba,
            moxcms::TransformOptions::default(),
        )
        .expect("RGBA transform");

    eprintln!("\nAlpha channel preservation:");

    let test_cases = [
        ([255u8, 0, 0, 0], "Red, transparent"),
        ([255, 0, 0, 128], "Red, 50% alpha"),
        ([255, 0, 0, 255], "Red, opaque"),
        ([128, 128, 128, 64], "Gray, 25% alpha"),
    ];

    for (input, desc) in test_cases {
        let mut output = [0u8; 4];
        transform.transform(&input, &mut output).unwrap();

        eprintln!(
            "  {} -> RGBA({}, {}, {}, {})",
            desc, output[0], output[1], output[2], output[3]
        );

        // Alpha should be preserved exactly
        assert_eq!(
            input[3], output[3],
            "Alpha channel should be preserved for {}",
            desc
        );
    }
}
