//! qcms parity tests
//!
//! Comprehensive tests comparing qcms (Firefox's CMS) with moxcms and lcms2.
//! qcms is a pure Rust color management library used in Firefox.

/// Test qcms profile creation
#[test]
fn test_qcms_profile_creation() {
    eprintln!("\nqcms profile creation:");

    // Create sRGB profile
    let _srgb = qcms::Profile::new_sRGB();
    eprintln!("  sRGB profile created successfully");

    // Create grayscale profile
    let _gray = qcms::Profile::new_gray_with_gamma(2.2);
    eprintln!("  Gray profile created");

    // Create XYZ D50 profile
    let _xyz = qcms::Profile::new_XYZD50();
    eprintln!("  XYZ D50 profile created");
}

/// Test qcms sRGB identity transform
#[test]
fn test_qcms_srgb_identity() {
    let srgb = qcms::Profile::new_sRGB();

    // Create identity transform
    let transform =
        qcms::Transform::new(&srgb, &srgb, qcms::DataType::RGB8, qcms::Intent::Perceptual);

    assert!(transform.is_some(), "Should create identity transform");
    let transform = transform.unwrap();

    eprintln!("\nqcms sRGB identity transform:");

    // Test various colors
    let test_colors: Vec<[u8; 3]> = vec![
        [0, 0, 0],       // Black
        [255, 255, 255], // White
        [255, 0, 0],     // Red
        [0, 255, 0],     // Green
        [0, 0, 255],     // Blue
        [128, 128, 128], // Gray
        [200, 100, 50],  // Random
    ];

    for color in &test_colors {
        let mut data = color.to_vec();
        transform.apply(&mut data);

        let diff = (0..3)
            .map(|i| (color[i] as i32 - data[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!("  {:?} -> {:?} (diff: {})", color, &data[..3], diff);

        assert!(
            diff <= 1,
            "Identity transform should preserve colors: {:?} -> {:?}",
            color,
            &data[..3]
        );
    }
}

/// Compare qcms with lcms2 for sRGB identity
#[test]
fn test_qcms_vs_lcms2_srgb_identity() {
    let qcms_srgb = qcms::Profile::new_sRGB();
    let lcms_srgb = lcms2::Profile::new_srgb();

    let qcms_transform = qcms::Transform::new(
        &qcms_srgb,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )
    .expect("qcms transform");

    let lcms_transform = lcms2::Transform::new(
        &lcms_srgb,
        lcms2::PixelFormat::RGB_8,
        &lcms_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .expect("lcms2 transform");

    eprintln!("\nqcms vs lcms2 sRGB identity:");

    // Test with color cube
    let test_values: Vec<u8> = (0..=255).step_by(17).map(|v| v as u8).collect();
    let mut max_diff = 0i32;

    for &r in &test_values {
        for &g in &test_values {
            for &b in &test_values {
                // qcms (in-place)
                let mut qcms_data = vec![r, g, b];
                qcms_transform.apply(&mut qcms_data);

                // lcms2
                let input = [r, g, b];
                let mut lcms_output = [0u8; 3];
                lcms_transform.transform_pixels(&input, &mut lcms_output);

                for i in 0..3 {
                    let diff = (qcms_data[i] as i32 - lcms_output[i] as i32).abs();
                    max_diff = max_diff.max(diff);
                }
            }
        }
    }

    eprintln!("  Max channel difference: {}", max_diff);

    assert!(
        max_diff <= 1,
        "qcms and lcms2 should match for sRGB identity"
    );
}

/// Compare qcms with moxcms for sRGB identity
#[test]
fn test_qcms_vs_moxcms_srgb_identity() {
    let qcms_srgb = qcms::Profile::new_sRGB();
    let mox_srgb = moxcms::ColorProfile::new_srgb();

    let qcms_transform = qcms::Transform::new(
        &qcms_srgb,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )
    .expect("qcms transform");

    let mox_transform = mox_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &mox_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("moxcms transform");

    eprintln!("\nqcms vs moxcms sRGB identity:");

    // Test with color cube
    let test_values: Vec<u8> = (0..=255).step_by(17).map(|v| v as u8).collect();
    let mut max_diff = 0i32;

    for &r in &test_values {
        for &g in &test_values {
            for &b in &test_values {
                // qcms (in-place)
                let mut qcms_data = vec![r, g, b];
                qcms_transform.apply(&mut qcms_data);

                // moxcms
                let input = [r, g, b];
                let mut mox_output = [0u8; 3];
                mox_transform.transform(&input, &mut mox_output).unwrap();

                for i in 0..3 {
                    let diff = (qcms_data[i] as i32 - mox_output[i] as i32).abs();
                    max_diff = max_diff.max(diff);
                }
            }
        }
    }

    eprintln!("  Max channel difference: {}", max_diff);

    assert!(
        max_diff <= 1,
        "qcms and moxcms should match for sRGB identity"
    );
}

/// Compare all three CMS implementations
#[test]
fn test_all_three_cms_comparison() {
    let qcms_srgb = qcms::Profile::new_sRGB();
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let lcms_srgb = lcms2::Profile::new_srgb();

    let qcms_transform = qcms::Transform::new(
        &qcms_srgb,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )
    .expect("qcms transform");

    let mox_transform = mox_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &mox_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
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

    eprintln!("\nThree-way CMS comparison (sRGB identity):");
    eprintln!("  Testing full color cube...");

    let test_values: Vec<u8> = (0..=255).step_by(17).map(|v| v as u8).collect();
    let mut qcms_vs_mox_max = 0i32;
    let mut qcms_vs_lcms_max = 0i32;
    let mut mox_vs_lcms_max = 0i32;

    for &r in &test_values {
        for &g in &test_values {
            for &b in &test_values {
                // qcms
                let mut qcms_data = vec![r, g, b];
                qcms_transform.apply(&mut qcms_data);

                // moxcms
                let input = [r, g, b];
                let mut mox_output = [0u8; 3];
                mox_transform.transform(&input, &mut mox_output).unwrap();

                // lcms2
                let mut lcms_output = [0u8; 3];
                lcms_transform.transform_pixels(&input, &mut lcms_output);

                for i in 0..3 {
                    qcms_vs_mox_max =
                        qcms_vs_mox_max.max((qcms_data[i] as i32 - mox_output[i] as i32).abs());
                    qcms_vs_lcms_max =
                        qcms_vs_lcms_max.max((qcms_data[i] as i32 - lcms_output[i] as i32).abs());
                    mox_vs_lcms_max =
                        mox_vs_lcms_max.max((mox_output[i] as i32 - lcms_output[i] as i32).abs());
                }
            }
        }
    }

    eprintln!("  qcms vs moxcms: max diff = {}", qcms_vs_mox_max);
    eprintln!("  qcms vs lcms2:  max diff = {}", qcms_vs_lcms_max);
    eprintln!("  moxcms vs lcms2: max diff = {}", mox_vs_lcms_max);

    assert!(
        qcms_vs_mox_max <= 1,
        "qcms and moxcms should match within 1 level"
    );
    assert!(
        qcms_vs_lcms_max <= 1,
        "qcms and lcms2 should match within 1 level"
    );
    assert!(
        mox_vs_lcms_max <= 1,
        "moxcms and lcms2 should match within 1 level"
    );
}

/// Test qcms rendering intents
#[test]
fn test_qcms_rendering_intents() {
    let srgb = qcms::Profile::new_sRGB();

    let intents = [
        (qcms::Intent::Perceptual, "Perceptual"),
        (qcms::Intent::RelativeColorimetric, "Relative Colorimetric"),
        (qcms::Intent::Saturation, "Saturation"),
        (qcms::Intent::AbsoluteColorimetric, "Absolute Colorimetric"),
    ];

    eprintln!("\nqcms rendering intents:");

    for (intent, name) in &intents {
        let transform = qcms::Transform::new(&srgb, &srgb, qcms::DataType::RGB8, *intent);

        match transform {
            Some(t) => {
                let mut data = vec![200u8, 100, 50];
                t.apply(&mut data);
                eprintln!("  {}: {:?} (created OK)", name, &data[..3]);
            }
            None => {
                eprintln!("  {}: FAILED to create transform", name);
            }
        }
    }
}

/// Test qcms RGBA transform
#[test]
fn test_qcms_rgba_transform() {
    let srgb = qcms::Profile::new_sRGB();

    let transform = qcms::Transform::new(
        &srgb,
        &srgb,
        qcms::DataType::RGBA8,
        qcms::Intent::Perceptual,
    )
    .expect("RGBA transform");

    eprintln!("\nqcms RGBA transform:");

    // Test with various alpha values
    let test_cases = [
        [255u8, 0, 0, 0],    // Red, transparent
        [255, 0, 0, 128],    // Red, 50% alpha
        [255, 0, 0, 255],    // Red, opaque
        [128, 128, 128, 64], // Gray, 25% alpha
    ];

    for input in &test_cases {
        let mut data = input.to_vec();
        transform.apply(&mut data);

        eprintln!(
            "  RGBA({}, {}, {}, {}) -> ({}, {}, {}, {})",
            input[0], input[1], input[2], input[3], data[0], data[1], data[2], data[3]
        );

        // Alpha should be preserved
        assert_eq!(input[3], data[3], "Alpha channel should be preserved");
    }
}

/// Test qcms grayscale profile creation
/// Note: qcms supports grayscale profile creation but has limited grayscale transform support
#[test]
fn test_qcms_grayscale_profile() {
    eprintln!("\nqcms grayscale profile:");

    // qcms can create grayscale profiles
    let gray = qcms::Profile::new_gray_with_gamma(2.2);
    eprintln!("  Created gray profile with gamma 2.2");

    // But transforms involving grayscale are limited
    // Testing Gray8 causes panics because qcms expects matching output type
    // qcms's Transform::apply expects input buffer to match both input AND output profiles

    // For now, just verify we can create the profile
    let srgb = qcms::Profile::new_sRGB();
    eprintln!("  Gray profiles can be created");

    // qcms does NOT support:
    // - Gray8 input with RGB8 output (panics)
    // - Gray8 to Gray8 transforms panic too
    // This is a known limitation of qcms compared to lcms2/moxcms

    // Test that we CAN use grayscale as part of ICC profile parsing
    // by loading an actual grayscale profile
    use std::path::Path;
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
        .join("profiles");

    // Note: We'd need an actual grayscale ICC profile to test this properly
    let gray_icc = testdata.join("gray.icc");
    if gray_icc.exists() {
        if let Ok(data) = std::fs::read(&gray_icc) {
            if let Some(_profile) = qcms::Profile::new_from_slice(&data, false) {
                eprintln!("  Loaded gray.icc successfully");
            }
        }
    }

    // Confirm both profiles were created (no panics)
    drop(gray);
    drop(srgb);
    eprintln!("  Profile creation: OK");
}

/// Test qcms with ICC profile from bytes
#[test]
fn test_qcms_icc_profile_parsing() {
    use std::path::Path;

    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
        .join("profiles");

    let srgb_path = testdata.join("sRGB.icc");

    eprintln!("\nqcms ICC profile parsing:");

    if srgb_path.exists() {
        let data = std::fs::read(&srgb_path).expect("read sRGB.icc");

        // qcms::Profile::new_from_slice(data, curves_only)
        match qcms::Profile::new_from_slice(&data, false) {
            Some(profile) => {
                eprintln!("  sRGB.icc: parsed successfully");

                // Try to create transform with it
                let srgb_builtin = qcms::Profile::new_sRGB();
                let transform = qcms::Transform::new(
                    &profile,
                    &srgb_builtin,
                    qcms::DataType::RGB8,
                    qcms::Intent::Perceptual,
                );

                match transform {
                    Some(t) => {
                        let mut data = vec![255u8, 128, 64];
                        t.apply(&mut data);
                        eprintln!("  Transform: [255,128,64] -> {:?}", &data[..3]);
                    }
                    None => {
                        eprintln!("  Transform: failed to create");
                    }
                }
            }
            None => {
                eprintln!("  sRGB.icc: failed to parse");
            }
        }
    } else {
        eprintln!("  Skipping: sRGB.icc not found");
    }
}

/// Compare qcms transform speed characteristics
#[test]
fn test_qcms_determinism() {
    let srgb = qcms::Profile::new_sRGB();
    let transform =
        qcms::Transform::new(&srgb, &srgb, qcms::DataType::RGB8, qcms::Intent::Perceptual)
            .expect("transform");

    eprintln!("\nqcms determinism:");

    // Run same transform multiple times
    let mut results = Vec::new();
    for _ in 0..10 {
        let mut data = vec![200u8, 100, 50];
        transform.apply(&mut data);
        results.push(data);
    }

    // All results should be identical
    let first = &results[0];
    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            first, result,
            "Transform iteration {} produced different result",
            i
        );
    }

    eprintln!("  10 iterations: all identical {:?}", first);
}

/// Test qcms with all rendering intents comparing to lcms2
#[test]
fn test_qcms_all_intents_vs_lcms2() {
    let qcms_srgb = qcms::Profile::new_sRGB();
    let lcms_srgb = lcms2::Profile::new_srgb();

    let intents = [
        (
            qcms::Intent::Perceptual,
            lcms2::Intent::Perceptual,
            "Perceptual",
        ),
        (
            qcms::Intent::RelativeColorimetric,
            lcms2::Intent::RelativeColorimetric,
            "Relative",
        ),
        (
            qcms::Intent::Saturation,
            lcms2::Intent::Saturation,
            "Saturation",
        ),
        (
            qcms::Intent::AbsoluteColorimetric,
            lcms2::Intent::AbsoluteColorimetric,
            "Absolute",
        ),
    ];

    eprintln!("\nqcms vs lcms2 all intents:");

    for (qcms_intent, lcms_intent, name) in &intents {
        let qcms_transform =
            qcms::Transform::new(&qcms_srgb, &qcms_srgb, qcms::DataType::RGB8, *qcms_intent);

        let lcms_transform = lcms2::Transform::new(
            &lcms_srgb,
            lcms2::PixelFormat::RGB_8,
            &lcms_srgb,
            lcms2::PixelFormat::RGB_8,
            *lcms_intent,
        );

        match (qcms_transform, lcms_transform) {
            (Some(qt), Ok(lt)) => {
                let input = [200u8, 100, 50];

                let mut qcms_data = input.to_vec();
                qt.apply(&mut qcms_data);

                let mut lcms_output = [0u8; 3];
                lt.transform_pixels(&input, &mut lcms_output);

                let max_diff = (0..3)
                    .map(|i| (qcms_data[i] as i32 - lcms_output[i] as i32).abs())
                    .max()
                    .unwrap();

                eprintln!(
                    "  {}: qcms {:?}, lcms2 {:?}, diff: {}",
                    name,
                    &qcms_data[..3],
                    lcms_output,
                    max_diff
                );

                assert!(
                    max_diff <= 1,
                    "{} intent should match between qcms and lcms2",
                    name
                );
            }
            _ => {
                eprintln!("  {}: one or both transforms failed", name);
            }
        }
    }
}
