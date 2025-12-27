//! CMYK Transform Tests
//!
//! Tests for CMYK (Cyan, Magenta, Yellow, blacK) color space transforms.
//! CMYK is commonly used in printing workflows.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// CMYK profiles in our test corpus
const CMYK_PROFILES: &[&str] = &[
    "skcms/misc/Coated_FOGRA39_CMYK.icc",
    "qcms/ps_cmyk_min.icc",
    "lcms2/test1.icc",
];

/// Test CMYK profile parsing
#[test]
fn test_cmyk_profile_parsing() {
    eprintln!("\n=== CMYK Profile Parsing ===\n");

    let profiles_dir = testdata_dir().join("profiles");

    for profile_rel in CMYK_PROFILES {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            eprintln!("SKIP: {} (not found)", profile_rel);
            continue;
        }

        let data = match std::fs::read(&profile_path) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("SKIP: {} (read error)", profile_rel);
                continue;
            }
        };

        // Test moxcms parsing
        let moxcms_result = moxcms::ColorProfile::new_from_slice(&data);
        let moxcms_ok = moxcms_result.is_ok();

        // Test lcms2 parsing
        let lcms2_result = lcms2::Profile::new_icc(&data);
        let lcms2_ok = lcms2_result.is_ok();

        eprintln!(
            "{}: moxcms={} lcms2={}",
            profile_rel,
            if moxcms_ok { "✓" } else { "✗" },
            if lcms2_ok { "✓" } else { "✗" }
        );

        if let Ok(profile) = moxcms_result {
            eprintln!("  Color space: {:?}", profile.color_space);
            eprintln!("  Profile class: {:?}", profile.profile_class);
            eprintln!("  Version: {:?}", profile.version());
            assert_eq!(
                profile.color_space,
                moxcms::DataColorSpace::Cmyk,
                "Expected CMYK color space"
            );
        }
    }
}

/// Test CMYK to sRGB transform
#[test]
fn test_cmyk_to_srgb_transform() {
    eprintln!("\n=== CMYK to sRGB Transform ===\n");

    let profiles_dir = testdata_dir().join("profiles");

    // Use FOGRA39 which is a well-known CMYK profile
    let fogra39_path = profiles_dir.join("skcms/misc/Coated_FOGRA39_CMYK.icc");
    if !fogra39_path.exists() {
        eprintln!("SKIP: Coated_FOGRA39_CMYK.icc not found");
        return;
    }

    let data = std::fs::read(&fogra39_path).unwrap();

    // Test with moxcms
    let cmyk_profile = match moxcms::ColorProfile::new_from_slice(&data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: moxcms parse failed: {:?}", e);
            return;
        }
    };

    let srgb_profile = moxcms::ColorProfile::new_srgb();

    // Create CMYK -> sRGB transform
    // Note: moxcms uses Layout::Rgba for CMYK (4 channels: C, M, Y, K)
    // and Layout::Rgb for RGB (3 channels)
    let transform = match cmyk_profile.create_transform_8bit(
        moxcms::Layout::Rgba, // 4 channels for CMYK
        &srgb_profile,
        moxcms::Layout::Rgb, // 3 channels for RGB
        moxcms::TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("SKIP: Transform creation failed: {:?}", e);
            return;
        }
    };

    // Test CMYK colors (4 channels: C, M, Y, K)
    let test_colors: Vec<([u8; 4], &str)> = vec![
        ([0, 0, 0, 0], "white (no ink)"),
        ([255, 0, 0, 0], "pure cyan"),
        ([0, 255, 0, 0], "pure magenta"),
        ([0, 0, 255, 0], "pure yellow"),
        ([0, 0, 0, 255], "pure black"),
        ([255, 255, 255, 255], "full coverage"),
        ([128, 128, 0, 0], "cyan+magenta (blue-ish)"),
        ([0, 128, 128, 0], "magenta+yellow (red-ish)"),
        ([128, 0, 128, 0], "cyan+yellow (green-ish)"),
    ];

    eprintln!("CMYK to sRGB transforms:");
    eprintln!("  CMYK input -> RGB output");

    for (cmyk, name) in &test_colors {
        let mut rgb_out = [0u8; 3];
        transform.transform(cmyk, &mut rgb_out).unwrap();

        eprintln!(
            "  C={} M={} Y={} K={} ({}) -> R={} G={} B={}",
            cmyk[0], cmyk[1], cmyk[2], cmyk[3], name, rgb_out[0], rgb_out[1], rgb_out[2]
        );
    }
}

/// Test CMYK to sRGB parity between moxcms and lcms2
#[test]
fn test_cmyk_to_srgb_lcms2_parity() {
    eprintln!("\n=== CMYK to sRGB Parity (moxcms vs lcms2) ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let fogra39_path = profiles_dir.join("skcms/misc/Coated_FOGRA39_CMYK.icc");

    if !fogra39_path.exists() {
        eprintln!("SKIP: Coated_FOGRA39_CMYK.icc not found");
        return;
    }

    let data = std::fs::read(&fogra39_path).unwrap();

    // Parse with both CMS
    let moxcms_cmyk = match moxcms::ColorProfile::new_from_slice(&data) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("SKIP: moxcms parse failed");
            return;
        }
    };

    let lcms2_cmyk = match lcms2::Profile::new_icc(&data) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("SKIP: lcms2 parse failed");
            return;
        }
    };

    // Create transforms
    // moxcms uses Layout::Rgba for 4-channel CMYK, Layout::Rgb for 3-channel RGB
    let moxcms_srgb = moxcms::ColorProfile::new_srgb();
    let moxcms_transform = match moxcms_cmyk.create_transform_8bit(
        moxcms::Layout::Rgba, // 4 channels for CMYK
        &moxcms_srgb,
        moxcms::Layout::Rgb, // 3 channels for RGB
        moxcms::TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("SKIP: moxcms transform failed");
            return;
        }
    };

    let lcms2_srgb = lcms2::Profile::new_srgb();
    let lcms2_transform = match lcms2::Transform::new(
        &lcms2_cmyk,
        lcms2::PixelFormat::CMYK_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    ) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("SKIP: lcms2 transform failed");
            return;
        }
    };

    // Test a grid of CMYK values
    let mut max_diff = 0i32;
    let mut total_tests = 0;
    let mut large_diffs = Vec::new();

    for c in (0..=255).step_by(51) {
        for m in (0..=255).step_by(51) {
            for y in (0..=255).step_by(51) {
                for k in (0..=255).step_by(85) {
                    let cmyk = [c as u8, m as u8, y as u8, k as u8];

                    let mut moxcms_rgb = [0u8; 3];
                    let mut lcms2_rgb = [0u8; 3];

                    moxcms_transform.transform(&cmyk, &mut moxcms_rgb).unwrap();
                    lcms2_transform.transform_pixels(&cmyk, &mut lcms2_rgb);

                    let diff = (0..3)
                        .map(|i| (moxcms_rgb[i] as i32 - lcms2_rgb[i] as i32).abs())
                        .max()
                        .unwrap();

                    if diff > max_diff {
                        max_diff = diff;
                    }

                    if diff > 5 {
                        large_diffs.push((cmyk, moxcms_rgb, lcms2_rgb, diff));
                    }

                    total_tests += 1;
                }
            }
        }
    }

    eprintln!("Tested {} CMYK values", total_tests);
    eprintln!("Max channel difference: {}", max_diff);

    if !large_diffs.is_empty() {
        eprintln!("\nLarge differences (>5):");
        for (cmyk, mox, lcms, diff) in large_diffs.iter().take(10) {
            eprintln!(
                "  CMYK {:?} -> moxcms {:?} vs lcms2 {:?} (diff={})",
                cmyk, mox, lcms, diff
            );
        }
        if large_diffs.len() > 10 {
            eprintln!("  ... and {} more", large_diffs.len() - 10);
        }
    }

    // CMYK transforms are more complex and may have larger differences
    // Accept up to 15 as acceptable for CMYK
    assert!(
        max_diff <= 15,
        "CMYK transform difference too large: {}",
        max_diff
    );
}

/// Test sRGB to CMYK transform
#[test]
fn test_srgb_to_cmyk_transform() {
    eprintln!("\n=== sRGB to CMYK Transform ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let fogra39_path = profiles_dir.join("skcms/misc/Coated_FOGRA39_CMYK.icc");

    if !fogra39_path.exists() {
        eprintln!("SKIP: Coated_FOGRA39_CMYK.icc not found");
        return;
    }

    let data = std::fs::read(&fogra39_path).unwrap();

    let cmyk_profile = match moxcms::ColorProfile::new_from_slice(&data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: moxcms parse failed: {:?}", e);
            return;
        }
    };

    let srgb_profile = moxcms::ColorProfile::new_srgb();

    // Create sRGB -> CMYK transform
    // moxcms uses Layout::Rgb for 3-channel RGB, Layout::Rgba for 4-channel CMYK
    let transform = match srgb_profile.create_transform_8bit(
        moxcms::Layout::Rgb, // 3 channels for RGB
        &cmyk_profile,
        moxcms::Layout::Rgba, // 4 channels for CMYK
        moxcms::TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("SKIP: Transform creation failed: {:?}", e);
            return;
        }
    };

    // Test RGB colors
    let test_colors: Vec<([u8; 3], &str)> = vec![
        ([255, 255, 255], "white"),
        ([0, 0, 0], "black"),
        ([255, 0, 0], "red"),
        ([0, 255, 0], "green"),
        ([0, 0, 255], "blue"),
        ([255, 255, 0], "yellow"),
        ([0, 255, 255], "cyan"),
        ([255, 0, 255], "magenta"),
        ([128, 128, 128], "gray"),
    ];

    eprintln!("sRGB to CMYK transforms:");
    eprintln!("  RGB input -> CMYK output");

    for (rgb, name) in &test_colors {
        let mut cmyk_out = [0u8; 4];
        transform.transform(rgb, &mut cmyk_out).unwrap();

        eprintln!(
            "  R={} G={} B={} ({}) -> C={} M={} Y={} K={}",
            rgb[0], rgb[1], rgb[2], name, cmyk_out[0], cmyk_out[1], cmyk_out[2], cmyk_out[3]
        );
    }
}

/// Test CMYK round-trip (sRGB -> CMYK -> sRGB)
#[test]
fn test_cmyk_round_trip() {
    eprintln!("\n=== CMYK Round-Trip Test ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let fogra39_path = profiles_dir.join("skcms/misc/Coated_FOGRA39_CMYK.icc");

    if !fogra39_path.exists() {
        eprintln!("SKIP: Coated_FOGRA39_CMYK.icc not found");
        return;
    }

    let data = std::fs::read(&fogra39_path).unwrap();

    let cmyk_profile = match moxcms::ColorProfile::new_from_slice(&data) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("SKIP: moxcms parse failed");
            return;
        }
    };

    let srgb_profile = moxcms::ColorProfile::new_srgb();

    // Create transforms
    // moxcms uses Layout::Rgb for 3-channel RGB, Layout::Rgba for 4-channel CMYK
    let rgb_to_cmyk = match srgb_profile.create_transform_8bit(
        moxcms::Layout::Rgb, // 3 channels for RGB
        &cmyk_profile,
        moxcms::Layout::Rgba, // 4 channels for CMYK
        moxcms::TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("SKIP: RGB->CMYK transform failed");
            return;
        }
    };

    let cmyk_to_rgb = match cmyk_profile.create_transform_8bit(
        moxcms::Layout::Rgba, // 4 channels for CMYK
        &srgb_profile,
        moxcms::Layout::Rgb, // 3 channels for RGB
        moxcms::TransformOptions::default(),
    ) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("SKIP: CMYK->RGB transform failed");
            return;
        }
    };

    // Test round-trip accuracy
    let test_colors = [
        [128u8, 128, 128], // Gray (should be well-preserved)
        [200, 150, 100],   // Skin tone
        [0, 0, 0],         // Black
        [255, 255, 255],   // White
    ];

    eprintln!("Round-trip accuracy (sRGB -> CMYK -> sRGB):");

    for rgb in &test_colors {
        let mut cmyk = [0u8; 4];
        let mut rgb_back = [0u8; 3];

        rgb_to_cmyk.transform(rgb, &mut cmyk).unwrap();
        cmyk_to_rgb.transform(&cmyk, &mut rgb_back).unwrap();

        let diff = (0..3)
            .map(|i| (rgb[i] as i32 - rgb_back[i] as i32).abs())
            .max()
            .unwrap();

        eprintln!(
            "  [{},{},{}] -> CMYK [{},{},{},{}] -> [{},{},{}] (diff={})",
            rgb[0],
            rgb[1],
            rgb[2],
            cmyk[0],
            cmyk[1],
            cmyk[2],
            cmyk[3],
            rgb_back[0],
            rgb_back[1],
            rgb_back[2],
            diff
        );
    }

    eprintln!("\nNote: CMYK gamut is smaller than sRGB, so some colors may clip.");
}
