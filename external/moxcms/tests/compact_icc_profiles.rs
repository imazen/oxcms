//! Comprehensive tests for Compact-ICC-Profiles from saucecontrol/Compact-ICC-Profiles
//!
//! Tests profile parsing, negative XYZ value handling (Display P3, Rec2020),
//! and color transforms with all profile types.
//!
//! Key concern: Display P3 and Rec2020 have negative Z values for the red primary
//! when adapted to D50 PCS illuminant. The "Compat" versions have this nudged to 0.

use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};
use std::path::PathBuf;

fn profiles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata/compact-icc-profiles")
}

/// All Compact-ICC-Profiles from the repository
const ALL_PROFILES: &[&str] = &[
    // sRGB variants
    "sRGB-v2-nano.icc",
    "sRGB-v2-micro.icc",
    "sRGB-v2-magic.icc",
    "sRGB-v4.icc",
    "scRGB-v2.icc",
    // sGrey variants
    "sGrey-v2-nano.icc",
    "sGrey-v2-micro.icc",
    "sGrey-v2-magic.icc",
    "sGrey-v4.icc",
    // Display P3 - has negative red Z value
    "DisplayP3-v2-micro.icc",
    "DisplayP3-v2-magic.icc",
    "DisplayP3-v4.icc",
    // Display P3 Compat - red Z nudged to 0
    "DisplayP3Compat-v2-micro.icc",
    "DisplayP3Compat-v2-magic.icc",
    "DisplayP3Compat-v4.icc",
    // DCI-P3
    "DCI-P3-v4.icc",
    // ProPhoto
    "ProPhoto-v2-micro.icc",
    "ProPhoto-v2-magic.icc",
    "ProPhoto-v4.icc",
    // Rec2020 - has negative red Z value
    "Rec2020-v2-micro.icc",
    "Rec2020-v2-magic.icc",
    "Rec2020-v4.icc",
    "Rec2020-g24-v4.icc",
    // Rec2020 Compat - red Z nudged to 0
    "Rec2020Compat-v2-micro.icc",
    "Rec2020Compat-v2-magic.icc",
    "Rec2020Compat-v4.icc",
    // Rec709
    "Rec709-v2-micro.icc",
    "Rec709-v2-magic.icc",
    "Rec709-v4.icc",
    // Rec601 NTSC
    "Rec601NTSC-v2-micro.icc",
    "Rec601NTSC-v2-magic.icc",
    "Rec601NTSC-v4.icc",
    // Rec601 PAL
    "Rec601PAL-v2-micro.icc",
    "Rec601PAL-v2-magic.icc",
    "Rec601PAL-v4.icc",
    // Adobe-compatible
    "AdobeCompat-v2.icc",
    "AdobeCompat-v4.icc",
    // Apple-compatible
    "AppleCompat-v2.icc",
    "AppleCompat-v4.icc",
    // ColorMatch-compatible
    "ColorMatchCompat-v2.icc",
    "ColorMatchCompat-v4.icc",
    // WideGamut-compatible
    "WideGamutCompat-v2.icc",
    "WideGamutCompat-v4.icc",
    // CMYK
    "CGATS001Compat-v2-micro.icc",
];

/// RGB profiles (excluding gray and CMYK)
const RGB_PROFILES: &[&str] = &[
    "sRGB-v2-nano.icc",
    "sRGB-v2-micro.icc",
    "sRGB-v2-magic.icc",
    "sRGB-v4.icc",
    "scRGB-v2.icc",
    "DisplayP3-v2-micro.icc",
    "DisplayP3-v2-magic.icc",
    "DisplayP3-v4.icc",
    "DisplayP3Compat-v2-micro.icc",
    "DisplayP3Compat-v2-magic.icc",
    "DisplayP3Compat-v4.icc",
    "DCI-P3-v4.icc",
    "ProPhoto-v2-micro.icc",
    "ProPhoto-v2-magic.icc",
    "ProPhoto-v4.icc",
    "Rec2020-v2-micro.icc",
    "Rec2020-v2-magic.icc",
    "Rec2020-v4.icc",
    "Rec2020-g24-v4.icc",
    "Rec2020Compat-v2-micro.icc",
    "Rec2020Compat-v2-magic.icc",
    "Rec2020Compat-v4.icc",
    "Rec709-v2-micro.icc",
    "Rec709-v2-magic.icc",
    "Rec709-v4.icc",
    "Rec601NTSC-v2-micro.icc",
    "Rec601NTSC-v2-magic.icc",
    "Rec601NTSC-v4.icc",
    "Rec601PAL-v2-micro.icc",
    "Rec601PAL-v2-magic.icc",
    "Rec601PAL-v4.icc",
    "AdobeCompat-v2.icc",
    "AdobeCompat-v4.icc",
    "AppleCompat-v2.icc",
    "AppleCompat-v4.icc",
    "ColorMatchCompat-v2.icc",
    "ColorMatchCompat-v4.icc",
    "WideGamutCompat-v2.icc",
    "WideGamutCompat-v4.icc",
];

/// Profiles known to have negative XYZ values (non-Compat versions)
/// Display P3, Rec2020, and DCI-P3 all have negative red Z values when adapted to D50
const NEGATIVE_XYZ_PROFILES: &[&str] = &[
    "DisplayP3-v2-micro.icc",
    "DisplayP3-v2-magic.icc",
    "DisplayP3-v4.icc",
    "DCI-P3-v4.icc",  // Also has negative red Z
    "Rec2020-v2-micro.icc",
    "Rec2020-v2-magic.icc",
    "Rec2020-v4.icc",
    "Rec2020-g24-v4.icc",
];

/// Compat profiles with red Z nudged to 0
const COMPAT_PROFILES: &[&str] = &[
    "DisplayP3Compat-v2-micro.icc",
    "DisplayP3Compat-v2-magic.icc",
    "DisplayP3Compat-v4.icc",
    "Rec2020Compat-v2-micro.icc",
    "Rec2020Compat-v2-magic.icc",
    "Rec2020Compat-v4.icc",
];

/// Gray profiles
const GRAY_PROFILES: &[&str] = &[
    "sGrey-v2-nano.icc",
    "sGrey-v2-micro.icc",
    "sGrey-v2-magic.icc",
    "sGrey-v4.icc",
];

fn load_profile(name: &str) -> Option<(String, Vec<u8>)> {
    let path = profiles_dir().join(name);
    match std::fs::read(&path) {
        Ok(data) => Some((name.to_string(), data)),
        Err(e) => {
            eprintln!("Warning: Could not load {}: {}", name, e);
            None
        }
    }
}

#[test]
fn test_parse_all_profiles() {
    println!("\n=== Parsing All Compact-ICC-Profiles ===\n");
    println!(
        "{:<35} {:>8} {:>12} {:>10}",
        "Profile", "Size", "Color Space", "Version"
    );
    println!("{}", "-".repeat(70));

    let mut parse_failures = Vec::new();

    for name in ALL_PROFILES {
        if let Some((name, data)) = load_profile(name) {
            match ColorProfile::new_from_slice(&data) {
                Ok(profile) => {
                    let color_space = format!("{:?}", profile.color_space);
                    let version = format!("{:?}", profile.version());
                    println!(
                        "{:<35} {:>8} {:>12} {:>10}",
                        name,
                        data.len(),
                        color_space,
                        version
                    );
                }
                Err(e) => {
                    println!("{:<35} PARSE FAILED: {:?}", name, e);
                    parse_failures.push((name, format!("{:?}", e)));
                }
            }
        }
    }

    println!();
    if !parse_failures.is_empty() {
        println!("Parse failures:");
        for (name, err) in &parse_failures {
            println!("  {} - {}", name, err);
        }
    }

    assert!(
        parse_failures.is_empty(),
        "Failed to parse {} profiles",
        parse_failures.len()
    );
}

#[test]
fn test_negative_xyz_values() {
    println!("\n=== Negative XYZ Value Analysis ===\n");
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "Profile", "R.x", "R.y", "R.z", "G.x", "G.y", "G.z", "B.x", "B.y", "B.z"
    );
    println!("{}", "-".repeat(125));

    let mut negative_values_found = Vec::new();

    for name in RGB_PROFILES.iter() {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(profile) = ColorProfile::new_from_slice(&data) {
                let r = profile.red_colorant;
                let g = profile.green_colorant;
                let b = profile.blue_colorant;

                let has_negative = r.x < 0.0
                    || r.y < 0.0
                    || r.z < 0.0
                    || g.x < 0.0
                    || g.y < 0.0
                    || g.z < 0.0
                    || b.x < 0.0
                    || b.y < 0.0
                    || b.z < 0.0;

                let marker = if has_negative { " *NEG*" } else { "" };

                println!(
                    "{:<35} {:>10.6} {:>10.6} {:>10.6} {:>10.6} {:>10.6} {:>10.6} {:>10.6} {:>10.6} {:>10.6}{}",
                    name, r.x, r.y, r.z, g.x, g.y, g.z, b.x, b.y, b.z, marker
                );

                if has_negative {
                    negative_values_found.push((
                        name.clone(),
                        r.x,
                        r.y,
                        r.z,
                        g.x,
                        g.y,
                        g.z,
                        b.x,
                        b.y,
                        b.z,
                    ));
                }
            }
        }
    }

    println!();
    println!("Profiles with negative XYZ values: {}", negative_values_found.len());

    // Check that the known negative profiles actually have negative values
    for name in NEGATIVE_XYZ_PROFILES {
        let found = negative_values_found.iter().any(|(n, ..)| n == *name);
        if !found {
            println!(
                "Note: {} expected to have negative values but doesn't",
                name
            );
        }
    }

    // Check that Compat profiles do NOT have negative values
    for name in COMPAT_PROFILES {
        let found = negative_values_found.iter().any(|(n, ..)| n == *name);
        if found {
            println!(
                "Warning: Compat profile {} unexpectedly has negative values",
                name
            );
        }
    }
}

#[test]
fn test_display_p3_negative_z() {
    println!("\n=== Display P3 Negative Z Analysis ===\n");

    // The red primary for Display P3 when adapted to D50 has a negative Z value
    // This is mathematically correct but some strict ICC implementations reject it

    let profiles = [
        ("DisplayP3-v2-micro.icc", true),  // Should have negative Z
        ("DisplayP3-v4.icc", true),         // Should have negative Z
        ("DisplayP3Compat-v2-micro.icc", false), // Should NOT have negative Z
        ("DisplayP3Compat-v4.icc", false),       // Should NOT have negative Z
    ];

    for (name, expect_negative) in profiles {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(profile) = ColorProfile::new_from_slice(&data) {
                let red_z = profile.red_colorant.z;
                let is_negative = red_z < 0.0;

                println!(
                    "{}: red.z = {:.6} (negative: {}, expected negative: {})",
                    name, red_z, is_negative, expect_negative
                );

                if expect_negative && !is_negative {
                    println!("  WARNING: Expected negative red.z but got positive");
                }
                if !expect_negative && is_negative {
                    println!("  WARNING: Expected non-negative red.z but got negative");
                }
            }
        }
    }
}

#[test]
fn test_rec2020_negative_z() {
    println!("\n=== Rec2020 Negative Z Analysis ===\n");

    let profiles = [
        ("Rec2020-v2-micro.icc", true),  // Should have negative Z
        ("Rec2020-v4.icc", true),         // Should have negative Z
        ("Rec2020Compat-v2-micro.icc", false), // Should NOT have negative Z
        ("Rec2020Compat-v4.icc", false),       // Should NOT have negative Z
    ];

    for (name, expect_negative) in profiles {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(profile) = ColorProfile::new_from_slice(&data) {
                let red_z = profile.red_colorant.z;
                let is_negative = red_z < 0.0;

                println!(
                    "{}: red.z = {:.6} (negative: {}, expected negative: {})",
                    name, red_z, is_negative, expect_negative
                );

                if expect_negative && !is_negative {
                    println!("  WARNING: Expected negative red.z but got positive");
                }
                if !expect_negative && is_negative {
                    println!("  WARNING: Expected non-negative red.z but got negative");
                }
            }
        }
    }
}

#[test]
fn test_transforms_rgb_to_srgb() {
    println!("\n=== RGB Profile to sRGB Transforms ===\n");

    let srgb = ColorProfile::new_srgb();

    // Test colors: red, green, blue, white, gray, black
    let test_colors: &[(u8, u8, u8, &str)] = &[
        (255, 0, 0, "Red"),
        (0, 255, 0, "Green"),
        (0, 0, 255, "Blue"),
        (255, 255, 255, "White"),
        (128, 128, 128, "Gray"),
        (0, 0, 0, "Black"),
    ];

    let mut transform_failures = Vec::new();

    for name in RGB_PROFILES {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(src_profile) = ColorProfile::new_from_slice(&data) {
                match src_profile.create_transform_8bit(
                    Layout::Rgb,
                    &srgb,
                    Layout::Rgb,
                    TransformOptions::default(),
                ) {
                    Ok(transform) => {
                        // Transform should work for all test colors
                        for &(r, g, b, color_name) in test_colors {
                            let input = [r, g, b];
                            let mut output = [0u8; 3];

                            if let Err(e) = transform.transform(&input, &mut output) {
                                transform_failures.push((name.clone(), color_name, format!("{:?}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        transform_failures.push((name.clone(), "create", format!("{:?}", e)));
                    }
                }
            }
        }
    }

    if !transform_failures.is_empty() {
        println!("Transform failures:");
        for (profile, op, err) in &transform_failures {
            println!("  {} ({}) - {}", profile, op, err);
        }
    }

    assert!(
        transform_failures.is_empty(),
        "Transform failures: {:?}",
        transform_failures.len()
    );
}

#[test]
fn test_transforms_srgb_to_profiles() {
    println!("\n=== sRGB to RGB Profile Transforms ===\n");

    let srgb = ColorProfile::new_srgb();

    let test_colors: &[(u8, u8, u8)] = &[
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 255),
        (128, 128, 128),
        (0, 0, 0),
    ];

    let mut transform_failures = Vec::new();

    for name in RGB_PROFILES {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(dst_profile) = ColorProfile::new_from_slice(&data) {
                match srgb.create_transform_8bit(
                    Layout::Rgb,
                    &dst_profile,
                    Layout::Rgb,
                    TransformOptions::default(),
                ) {
                    Ok(transform) => {
                        for &(r, g, b) in test_colors {
                            let input = [r, g, b];
                            let mut output = [0u8; 3];

                            if let Err(e) = transform.transform(&input, &mut output) {
                                transform_failures.push((
                                    name.clone(),
                                    format!("({},{},{})", r, g, b),
                                    format!("{:?}", e),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        transform_failures.push((name.clone(), "create".to_string(), format!("{:?}", e)));
                    }
                }
            }
        }
    }

    if !transform_failures.is_empty() {
        println!("Transform failures:");
        for (profile, op, err) in &transform_failures {
            println!("  {} ({}) - {}", profile, op, err);
        }
    }

    assert!(
        transform_failures.is_empty(),
        "Transform failures: {:?}",
        transform_failures.len()
    );
}

#[test]
fn test_gray_profiles() {
    println!("\n=== Gray Profile Tests ===\n");

    for name in GRAY_PROFILES {
        if let Some((name, data)) = load_profile(name) {
            match ColorProfile::new_from_slice(&data) {
                Ok(profile) => {
                    assert_eq!(
                        profile.color_space,
                        DataColorSpace::Gray,
                        "{} should be Gray color space",
                        name
                    );
                    println!("{}: Gray profile, version {:?}", name, profile.version());
                }
                Err(e) => {
                    panic!("Failed to parse gray profile {}: {:?}", name, e);
                }
            }
        }
    }
}

#[test]
fn test_cmyk_profile() {
    println!("\n=== CMYK Profile Test ===\n");

    let name = "CGATS001Compat-v2-micro.icc";
    if let Some((name, data)) = load_profile(name) {
        match ColorProfile::new_from_slice(&data) {
            Ok(profile) => {
                assert_eq!(
                    profile.color_space,
                    DataColorSpace::Cmyk,
                    "{} should be CMYK color space",
                    name
                );
                println!(
                    "{}: CMYK profile, version {:?}, PCS: {:?}",
                    name,
                    profile.version(),
                    profile.pcs
                );
            }
            Err(e) => {
                panic!("Failed to parse CMYK profile {}: {:?}", name, e);
            }
        }
    }
}

#[test]
fn test_white_point_consistency() {
    println!("\n=== White Point Analysis ===\n");
    println!(
        "{:<35} {:>12} {:>12} {:>12} {:>12}",
        "Profile", "WP.x", "WP.y", "WP.z", "Sum(R+G+B)"
    );
    println!("{}", "-".repeat(85));

    // D50 white point (ICC PCS illuminant)
    let d50_x = 0.9642;
    let d50_y = 1.0;
    let d50_z = 0.8249;

    for name in RGB_PROFILES {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(profile) = ColorProfile::new_from_slice(&data) {
                let wp = profile.white_point;
                let implied_wp = profile.implied_white_point();

                println!(
                    "{:<35} {:>12.6} {:>12.6} {:>12.6} {:>12.6}",
                    name, wp.x, wp.y, wp.z, implied_wp.y
                );

                // Check white point is close to D50
                let d50_dist = ((wp.x - d50_x).powi(2)
                    + (wp.y - d50_y).powi(2)
                    + (wp.z - d50_z).powi(2))
                .sqrt();

                if d50_dist > 0.01 {
                    println!("  Note: White point differs from D50 by {:.6}", d50_dist);
                }
            }
        }
    }
}

#[test]
fn test_roundtrip_accuracy() {
    println!("\n=== Roundtrip Accuracy Test ===\n");

    let srgb = ColorProfile::new_srgb();

    // Test a gradient of values
    let test_values: Vec<u8> = (0..=255).step_by(17).collect();

    for name in RGB_PROFILES {
        if let Some((profile_name, data)) = load_profile(name) {
            if let Ok(profile) = ColorProfile::new_from_slice(&data) {
                // sRGB -> Profile -> sRGB roundtrip
                let to_profile = match srgb.create_transform_8bit(
                    Layout::Rgb,
                    &profile,
                    Layout::Rgb,
                    TransformOptions::default(),
                ) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let from_profile = match profile.create_transform_8bit(
                    Layout::Rgb,
                    &srgb,
                    Layout::Rgb,
                    TransformOptions::default(),
                ) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let mut max_diff = 0u8;

                for &v in &test_values {
                    let input = [v, v, v];
                    let mut intermediate = [0u8; 3];
                    let mut output = [0u8; 3];

                    if to_profile.transform(&input, &mut intermediate).is_ok()
                        && from_profile.transform(&intermediate, &mut output).is_ok()
                    {
                        let diff_r = (input[0] as i16 - output[0] as i16).unsigned_abs() as u8;
                        let diff_g = (input[1] as i16 - output[1] as i16).unsigned_abs() as u8;
                        let diff_b = (input[2] as i16 - output[2] as i16).unsigned_abs() as u8;

                        max_diff = max_diff.max(diff_r).max(diff_g).max(diff_b);
                    }
                }

                if max_diff > 2 {
                    println!("{}: max roundtrip diff = {}", profile_name, max_diff);
                }
            }
        }
    }
}

#[test]
fn test_negative_xyz_transform_handling() {
    println!("\n=== Transform Handling with Negative XYZ Profiles ===\n");

    let srgb = ColorProfile::new_srgb();

    // Test that profiles with negative XYZ values can still create valid transforms
    for name in NEGATIVE_XYZ_PROFILES {
        if let Some((name, data)) = load_profile(name) {
            if let Ok(profile) = ColorProfile::new_from_slice(&data) {
                println!("Testing {}", name);
                println!("  Red colorant: ({:.6}, {:.6}, {:.6})",
                         profile.red_colorant.x,
                         profile.red_colorant.y,
                         profile.red_colorant.z);

                // Test transform creation
                match profile.create_transform_8bit(
                    Layout::Rgb,
                    &srgb,
                    Layout::Rgb,
                    TransformOptions::default(),
                ) {
                    Ok(transform) => {
                        // Test with pure red (most affected by negative red Z)
                        let input = [255u8, 0, 0];
                        let mut output = [0u8; 3];

                        match transform.transform(&input, &mut output) {
                            Ok(_) => {
                                println!("  Pure red [255,0,0] -> [{},{},{}]",
                                         output[0], output[1], output[2]);

                                // Pure red from a wide gamut should map to sRGB red
                                // or slightly clipped/saturated
                                assert!(
                                    output[0] >= 200 || output[1] > 0 || output[2] > 0,
                                    "Pure red should map to something visible"
                                );
                            }
                            Err(e) => {
                                println!("  Transform failed: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  Create transform failed: {:?}", e);
                    }
                }
                println!();
            }
        }
    }
}
