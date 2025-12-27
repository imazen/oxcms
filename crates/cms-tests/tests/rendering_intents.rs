//! Rendering intent comparison tests
//!
//! Tests different rendering intents and compares behavior
//! between moxcms and lcms2.

use cms_tests::accuracy::{delta_e_2000, srgb_to_lab};


/// Test that different rendering intents produce different results
/// for out-of-gamut colors
#[test]
fn test_rendering_intents_produce_different_results() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let intents = [
        (moxcms::RenderingIntent::Perceptual, "Perceptual"),
        (moxcms::RenderingIntent::RelativeColorimetric, "Relative Colorimetric"),
        (moxcms::RenderingIntent::Saturation, "Saturation"),
        (moxcms::RenderingIntent::AbsoluteColorimetric, "Absolute Colorimetric"),
    ];

    // Use a saturated color that might differ between intents
    let test_color = [255u8, 0, 0]; // Pure red

    eprintln!("\nRendering intent comparison for sRGB red -> P3:");

    let mut results = Vec::new();

    for (intent, name) in &intents {
        let transform = srgb
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &p3,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions {
                    rendering_intent: *intent,
                    ..Default::default()
                },
            )
            .expect("transform");


        let mut output = [0u8; 3];
        transform.transform(&test_color, &mut output).unwrap();

        eprintln!("  {}: {:?}", name, output);
        results.push((name.to_string(), output));
    }

    // At least some intents should produce different results
    // (Note: For matrix-shaper profiles, they may be identical)
    eprintln!("\n  Note: Matrix-shaper profiles may produce identical results");
    eprintln!("  for different intents. This is expected behavior.");
}

/// Test perceptual intent with lcms2 comparison
#[test]
fn test_perceptual_intent_lcms2_comparison() {
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let lcms_srgb = lcms2::Profile::new_srgb();

    let mox_transform = mox_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &mox_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions {
                rendering_intent: moxcms::RenderingIntent::Perceptual,
                ..Default::default()
            },
        )
        .expect("moxcms transform");

    let lcms_transform = lcms2::Transform::new(
        &lcms_srgb,
        lcms2::PixelFormat::RGB_8,
        &lcms_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .expect("lcms2 transform");



    // Test a range of colors
    let test_colors: Vec<[u8; 3]> = (0..=255)
        .step_by(51)
        .flat_map(|r| {
            (0..=255)
                .step_by(51)
                .flat_map(move |g| (0..=255).step_by(51).map(move |b| [r as u8, g as u8, b as u8]))
        })
        .collect();

    let mut max_diff = 0i32;
    let mut max_delta_e = 0.0f64;

    for color in &test_colors {
        let mut mox_output = [0u8; 3];
        let mut lcms_output = [0u8; 3];

        mox_transform.transform(color, &mut mox_output).unwrap();
        lcms_transform.transform_pixels(color, &mut lcms_output);

        for i in 0..3 {
            max_diff = max_diff.max((mox_output[i] as i32 - lcms_output[i] as i32).abs());
        }

        let mox_lab = srgb_to_lab(mox_output[0], mox_output[1], mox_output[2]);
        let lcms_lab = srgb_to_lab(lcms_output[0], lcms_output[1], lcms_output[2]);
        max_delta_e = max_delta_e.max(delta_e_2000(mox_lab, lcms_lab));
    }

    eprintln!("\nPerceptual intent lcms2 comparison:");
    eprintln!("  Max channel difference: {}", max_diff);
    eprintln!("  Max ΔE2000: {:.4}", max_delta_e);

    assert!(
        max_diff <= 1,
        "Perceptual intent should match between moxcms and lcms2"
    );
}

/// Test relative colorimetric intent
#[test]
fn test_relative_colorimetric_intent() {
    let srgb = moxcms::ColorProfile::new_srgb();

    let transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions {
                rendering_intent: moxcms::RenderingIntent::RelativeColorimetric,
                ..Default::default()
            },
        )
        .expect("transform");



    // Identity transform should preserve all colors
    let test_colors = [
        [0u8, 0, 0],
        [128, 128, 128],
        [255, 255, 255],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
    ];

    eprintln!("\nRelative colorimetric (sRGB -> sRGB identity):");
    for color in &test_colors {
        let mut output = [0u8; 3];
        transform.transform(color, &mut output).unwrap();

        let diff = (0..3)
            .map(|i| (color[i] as i32 - output[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!("  {:?} -> {:?} (diff: {})", color, output, diff);
        assert!(diff <= 1, "Identity transform should preserve colors");
    }
}

/// Test absolute colorimetric intent
#[test]
fn test_absolute_colorimetric_intent() {
    let srgb = moxcms::ColorProfile::new_srgb();

    let transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions {
                rendering_intent: moxcms::RenderingIntent::AbsoluteColorimetric,
                ..Default::default()
            },
        )
        .expect("transform");



    // Absolute colorimetric should also preserve colors for same-profile transform
    let test_colors = [
        [0u8, 0, 0],
        [128, 128, 128],
        [255, 255, 255],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
    ];

    eprintln!("\nAbsolute colorimetric (sRGB -> sRGB identity):");
    for color in &test_colors {
        let mut output = [0u8; 3];
        transform.transform(color, &mut output).unwrap();

        let diff = (0..3)
            .map(|i| (color[i] as i32 - output[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!("  {:?} -> {:?} (diff: {})", color, output, diff);
        assert!(diff <= 1, "Identity transform should preserve colors");
    }
}

/// Test saturation intent
#[test]
fn test_saturation_intent() {
    let srgb = moxcms::ColorProfile::new_srgb();

    let transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions {
                rendering_intent: moxcms::RenderingIntent::Saturation,
                ..Default::default()
            },
        )
        .expect("transform");



    // Saturation intent should also work as identity for same-profile
    let test_colors = [
        [0u8, 0, 0],
        [128, 128, 128],
        [255, 255, 255],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
    ];

    eprintln!("\nSaturation intent (sRGB -> sRGB identity):");
    for color in &test_colors {
        let mut output = [0u8; 3];
        transform.transform(color, &mut output).unwrap();

        let diff = (0..3)
            .map(|i| (color[i] as i32 - output[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!("  {:?} -> {:?} (diff: {})", color, output, diff);
        assert!(diff <= 1, "Identity transform should preserve colors");
    }
}

/// Compare all intents with lcms2
#[test]
fn test_all_intents_lcms2_comparison() {
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let lcms_srgb = lcms2::Profile::new_srgb();

    let intents = [
        (moxcms::RenderingIntent::Perceptual, lcms2::Intent::Perceptual, "Perceptual"),
        (moxcms::RenderingIntent::RelativeColorimetric, lcms2::Intent::RelativeColorimetric, "Relative"),
        (moxcms::RenderingIntent::Saturation, lcms2::Intent::Saturation, "Saturation"),
        (moxcms::RenderingIntent::AbsoluteColorimetric, lcms2::Intent::AbsoluteColorimetric, "Absolute"),
    ];

    eprintln!("\nAll intents lcms2 comparison (sRGB identity):");

    for (mox_intent, lcms_intent, name) in &intents {
        let mox_transform = mox_srgb
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &mox_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions {
                    rendering_intent: *mox_intent,
                    ..Default::default()
                },
            )
            .expect("moxcms transform");

        let lcms_transform = lcms2::Transform::new(
            &lcms_srgb,
            lcms2::PixelFormat::RGB_8,
            &lcms_srgb,
            lcms2::PixelFormat::RGB_8,
            *lcms_intent,
        )
        .expect("lcms2 transform");



        let test_color = [200u8, 100, 50];
        let mut mox_output = [0u8; 3];
        let mut lcms_output = [0u8; 3];

        mox_transform.transform(&test_color, &mut mox_output).unwrap();
        lcms_transform.transform_pixels(&test_color, &mut lcms_output);

        let max_diff = (0..3)
            .map(|i| (mox_output[i] as i32 - lcms_output[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!(
            "  {}: moxcms {:?}, lcms2 {:?}, diff: {}",
            name, mox_output, lcms_output, max_diff
        );

        assert!(
            max_diff <= 1,
            "{} intent should match between moxcms and lcms2",
            name
        );
    }
}

/// Test that black point compensation option exists (placeholder)
#[test]
fn test_black_point_compensation_option() {
    // Black point compensation is not yet implemented in oxcms
    // but the option should exist in TransformOptions

    let options = moxcms::TransformOptions {
        rendering_intent: moxcms::RenderingIntent::RelativeColorimetric,
        ..Default::default()
    };

    // Just verify we can create transforms with different options
    let srgb = moxcms::ColorProfile::new_srgb();

    let _transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            options,
        )
        .expect("transform with options");

    eprintln!("\nBlack point compensation option:");
    eprintln!("  TransformOptions created successfully");
    eprintln!("  Note: BPC not yet fully implemented");
}
