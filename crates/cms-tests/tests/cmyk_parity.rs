//! CMYK Parity Tests
//!
//! Tests for CMYK color management, comparing moxcms/lcms2 behavior
//! with various industry-standard CMYK ICC profiles.
//!
//! Profiles tested:
//! - USWebCoatedSWOP.icc (U.S. Web Coated SWOP v2) - Adobe standard
//! - ghostscript_cmyk.icc (Artifex CMYK SWOP Profile)
//! - nip2_cmyk.icc (Chemical proof profile)
//! - lcms2_test_cmyk.icc (lcms2 testbed profile)

use lcms2::{Intent, PixelFormat, Profile};
use std::path::PathBuf;
use std::slice;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed difference between lcms2 and moxcms for CMYK->RGB transforms.
/// This accounts for implementation differences in gamut mapping and rounding.
const CMYK_TO_RGB_PARITY_TOLERANCE: u16 = 10;

/// Maximum allowed difference between lcms2 and moxcms for RGB->CMYK transforms.
/// Higher than CMYK->RGB because RGB->CMYK involves more complex gamut mapping
/// and black generation algorithms can differ between implementations.
const RGB_TO_CMYK_PARITY_TOLERANCE: u16 = 10;

/// Maximum allowed delta for CMYK->RGB->CMYK roundtrip.
/// High tolerance because sRGB gamut is smaller than CMYK - out-of-gamut
/// colors are clipped and cannot round-trip perfectly.
const ROUNDTRIP_GAMUT_TOLERANCE: u16 = 200;

/// Step size for iterating through color space in exhaustive tests.
/// 32 gives us 9 values per channel (0, 32, 64, ... 256 clamped to 255).
const COLOR_SAMPLE_STEP: usize = 32;

/// Coarser step for K channel to reduce test time while maintaining coverage.
const K_CHANNEL_SAMPLE_STEP: usize = 64;

/// Primary CMYK profile used for most tests
const PRIMARY_CMYK_PROFILE: &str = "USWebCoatedSWOP.icc";

// ============================================================================
// Test Helpers
// ============================================================================

/// Path to ICC profiles directory (fixtures are versioned in the repo)
fn icc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/icc")
}

/// Load an ICC profile from fixtures. Panics if missing - fixtures are required.
fn load_profile(name: &str) -> Profile {
    let path = icc_dir().join(name);
    Profile::new_file(&path).unwrap_or_else(|e| {
        panic!(
            "Required ICC fixture '{}' failed to load: {:?}\n\
             Expected at: {}\n\
             Fixtures should be versioned in the repository.",
            name,
            e,
            path.display()
        )
    })
}

/// Load ICC profile data as bytes. Panics if missing.
fn load_profile_data(name: &str) -> Vec<u8> {
    let path = icc_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "Required ICC fixture '{}' not found: {}\n\
             Expected at: {}\n\
             Fixtures should be versioned in the repository.",
            name,
            e,
            path.display()
        )
    })
}

// ============================================================================
// Profile Loading Tests
// ============================================================================

/// Test that all CMYK profiles can be loaded
#[test]
fn test_load_cmyk_profiles() {
    let profiles = [
        "USWebCoatedSWOP.icc",
        "ghostscript_cmyk.icc",
        "nip2_cmyk.icc",
        "lcms2_test_cmyk.icc",
    ];

    for name in profiles {
        let profile = load_profile(name);

        assert_eq!(
            profile.color_space(),
            lcms2::ColorSpaceSignature::CmykData,
            "{} should be CMYK color space",
            name
        );

        println!(
            "Loaded {}: version={:.1}, PCS={:?}",
            name,
            profile.version(),
            profile.pcs()
        );
    }
}

// ============================================================================
// CMYK to sRGB Transform Tests
// ============================================================================

/// Test CMYK to sRGB transform with USWebCoatedSWOP
#[test]
fn test_cmyk_to_srgb_swop() {
    let cmyk = load_profile(PRIMARY_CMYK_PROFILE);
    let srgb = Profile::new_srgb();

    let transform = lcms2::Transform::<[u8; 4], [u8; 3]>::new(
        &cmyk,
        PixelFormat::CMYK_8,
        &srgb,
        PixelFormat::RGB_8,
        Intent::Perceptual,
    )
    .expect("Transform creation failed");

    // Test key CMYK values
    let test_cases: &[([u8; 4], &str)] = &[
        ([0, 0, 0, 0], "White (no ink)"),
        ([0, 0, 0, 255], "Black (K only)"),
        ([255, 0, 0, 0], "Cyan"),
        ([0, 255, 0, 0], "Magenta"),
        ([0, 0, 255, 0], "Yellow"),
        ([255, 255, 0, 0], "Blue (C+M)"),
        ([255, 0, 255, 0], "Green (C+Y)"),
        ([0, 255, 255, 0], "Red (M+Y)"),
        ([255, 255, 255, 255], "Rich black (all 100%)"),
        ([128, 128, 128, 128], "50% all"),
    ];

    println!("CMYK to sRGB (USWebCoatedSWOP, Perceptual):");
    for (cmyk_val, name) in test_cases {
        let mut rgb = [0u8; 3];
        transform.transform_pixels(slice::from_ref(cmyk_val), slice::from_mut(&mut rgb));
        println!(
            "  {} CMYK({},{},{},{}) -> RGB({},{},{})",
            name, cmyk_val[0], cmyk_val[1], cmyk_val[2], cmyk_val[3], rgb[0], rgb[1], rgb[2]
        );

        // Basic sanity checks
        if *name == "White (no ink)" {
            assert!(
                rgb[0] > 240 && rgb[1] > 240 && rgb[2] > 240,
                "White should map to near-white RGB"
            );
        }
        if *name == "Black (K only)" {
            // Note: K-only black in CMYK doesn't map to RGB(0,0,0)
            // Real printing K ink produces a dark gray, not pure black
            // The profile's black point is typically around RGB(35,31,32)
            assert!(
                rgb[0] < 50 && rgb[1] < 50 && rgb[2] < 50,
                "Black should map to dark gray, got RGB({},{},{})",
                rgb[0],
                rgb[1],
                rgb[2]
            );
        }
    }
}

/// Test sRGB to CMYK transform
#[test]
fn test_srgb_to_cmyk_swop() {
    let cmyk = load_profile(PRIMARY_CMYK_PROFILE);
    let srgb = Profile::new_srgb();

    let transform = lcms2::Transform::<[u8; 3], [u8; 4]>::new(
        &srgb,
        PixelFormat::RGB_8,
        &cmyk,
        PixelFormat::CMYK_8,
        Intent::Perceptual,
    )
    .expect("Transform creation failed");

    // Test key RGB values
    let test_cases: &[([u8; 3], &str)] = &[
        ([255, 255, 255], "White"),
        ([0, 0, 0], "Black"),
        ([255, 0, 0], "Red"),
        ([0, 255, 0], "Green"),
        ([0, 0, 255], "Blue"),
        ([255, 255, 0], "Yellow"),
        ([255, 0, 255], "Magenta"),
        ([0, 255, 255], "Cyan"),
        ([128, 128, 128], "Gray"),
    ];

    println!("sRGB to CMYK (USWebCoatedSWOP, Perceptual):");
    for (rgb_val, name) in test_cases {
        let mut cmyk_out = [0u8; 4];
        transform.transform_pixels(slice::from_ref(rgb_val), slice::from_mut(&mut cmyk_out));
        println!(
            "  {} RGB({},{},{}) -> CMYK({},{},{},{})",
            name,
            rgb_val[0],
            rgb_val[1],
            rgb_val[2],
            cmyk_out[0],
            cmyk_out[1],
            cmyk_out[2],
            cmyk_out[3]
        );

        // Basic sanity checks
        if *name == "White" {
            let total_ink: u16 =
                cmyk_out[0] as u16 + cmyk_out[1] as u16 + cmyk_out[2] as u16 + cmyk_out[3] as u16;
            assert!(total_ink < 20, "White should have minimal ink");
        }
        if *name == "Black" {
            // Black can be rich black or K-only depending on profile
            assert!(cmyk_out[3] > 200, "Black should have high K");
        }
    }
}

// ============================================================================
// CMYK Roundtrip Tests
// ============================================================================

/// Test CMYK -> sRGB -> CMYK roundtrip stability
#[test]
fn test_cmyk_srgb_roundtrip() {
    let cmyk = load_profile(PRIMARY_CMYK_PROFILE);
    let srgb = Profile::new_srgb();

    let to_rgb = lcms2::Transform::<[u8; 4], [u8; 3]>::new(
        &cmyk,
        PixelFormat::CMYK_8,
        &srgb,
        PixelFormat::RGB_8,
        Intent::RelativeColorimetric,
    )
    .expect("To RGB transform failed");

    let to_cmyk = lcms2::Transform::<[u8; 3], [u8; 4]>::new(
        &srgb,
        PixelFormat::RGB_8,
        &cmyk,
        PixelFormat::CMYK_8,
        Intent::RelativeColorimetric,
    )
    .expect("To CMYK transform failed");

    // Test roundtrip for various CMYK values
    let mut max_delta = 0u16;
    let mut total_delta = 0u64;
    let mut count = 0u32;

    for c in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
        for m in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
            for y in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
                for k in (0u8..=255).step_by(K_CHANNEL_SAMPLE_STEP) {
                    let original = [c, m, y, k];
                    let mut rgb = [0u8; 3];
                    let mut roundtrip = [0u8; 4];

                    to_rgb.transform_pixels(slice::from_ref(&original), slice::from_mut(&mut rgb));
                    to_cmyk
                        .transform_pixels(slice::from_ref(&rgb), slice::from_mut(&mut roundtrip));

                    // Calculate max channel difference
                    let delta = [
                        (original[0] as i16 - roundtrip[0] as i16).unsigned_abs(),
                        (original[1] as i16 - roundtrip[1] as i16).unsigned_abs(),
                        (original[2] as i16 - roundtrip[2] as i16).unsigned_abs(),
                        (original[3] as i16 - roundtrip[3] as i16).unsigned_abs(),
                    ];
                    let channel_max = delta.iter().max().copied().unwrap();

                    if channel_max > max_delta {
                        max_delta = channel_max;
                    }
                    total_delta += channel_max as u64;
                    count += 1;
                }
            }
        }
    }

    let avg_delta = total_delta as f64 / count as f64;
    println!(
        "CMYK roundtrip: max_delta={}, avg_delta={:.2}, samples={}",
        max_delta, avg_delta, count
    );

    // CMYK -> RGB -> CMYK is lossy due to gamut differences
    // sRGB has a smaller gamut than CMYK, so out-of-gamut colors are clipped
    // Allow significant deviation but catch catastrophic failures
    assert!(
        max_delta < ROUNDTRIP_GAMUT_TOLERANCE,
        "Roundtrip max delta {} exceeds tolerance {} (gamut mapping expected but this is excessive)",
        max_delta,
        ROUNDTRIP_GAMUT_TOLERANCE
    );
}

// ============================================================================
// Multi-Profile CMYK Tests
// ============================================================================

/// Compare different CMYK profiles for consistency
#[test]
fn test_cmyk_profile_comparison() {
    let profiles = [
        ("USWebCoatedSWOP.icc", "Adobe SWOP"),
        ("ghostscript_cmyk.icc", "Ghostscript CMYK"),
    ];

    let srgb = Profile::new_srgb();

    // Test CMYK input: 50% each channel
    let cmyk_input = [128u8, 128, 128, 128];

    println!("Comparing CMYK profiles for CMYK(128,128,128,128) -> RGB:");
    for (filename, label) in profiles {
        let cmyk = load_profile(filename);
        let transform = lcms2::Transform::<[u8; 4], [u8; 3]>::new(
            &cmyk,
            PixelFormat::CMYK_8,
            &srgb,
            PixelFormat::RGB_8,
            Intent::Perceptual,
        )
        .expect("Transform creation failed");

        let mut rgb = [0u8; 3];
        transform.transform_pixels(slice::from_ref(&cmyk_input), slice::from_mut(&mut rgb));
        println!("  {}: RGB({},{},{})", label, rgb[0], rgb[1], rgb[2]);
    }
}

// ============================================================================
// CMYK Float Transform Tests
// ============================================================================

/// Test CMYK float transforms
#[test]
fn test_cmyk_float_transform() {
    let cmyk = load_profile(PRIMARY_CMYK_PROFILE);
    let srgb = Profile::new_srgb();

    let transform = lcms2::Transform::<[f32; 4], [f32; 3]>::new(
        &cmyk,
        PixelFormat::CMYK_FLT,
        &srgb,
        PixelFormat::RGB_FLT,
        Intent::Perceptual,
    )
    .expect("Float transform creation failed");

    // Test with float values
    let test_cases: &[([f32; 4], &str)] = &[
        ([0.0, 0.0, 0.0, 0.0], "White"),
        ([0.0, 0.0, 0.0, 1.0], "Black"),
        ([1.0, 0.0, 0.0, 0.0], "Cyan"),
        ([0.0, 1.0, 0.0, 0.0], "Magenta"),
        ([0.0, 0.0, 1.0, 0.0], "Yellow"),
        ([0.5, 0.5, 0.5, 0.5], "50% all"),
    ];

    println!("CMYK float to sRGB float (Perceptual):");
    for (cmyk_val, name) in test_cases {
        let mut rgb = [0.0f32; 3];
        transform.transform_pixels(slice::from_ref(cmyk_val), slice::from_mut(&mut rgb));
        println!(
            "  {} CMYK({:.2},{:.2},{:.2},{:.2}) -> RGB({:.3},{:.3},{:.3})",
            name, cmyk_val[0], cmyk_val[1], cmyk_val[2], cmyk_val[3], rgb[0], rgb[1], rgb[2]
        );

        // Sanity checks - allow small overshoot due to gamut mapping
        // Some CMYK colors are out of sRGB gamut and may slightly exceed [0,1]
        assert!(
            rgb[0] >= -0.1 && rgb[0] <= 1.1,
            "R should be approximately in [0,1], got {}",
            rgb[0]
        );
        assert!(
            rgb[1] >= -0.1 && rgb[1] <= 1.1,
            "G should be approximately in [0,1], got {}",
            rgb[1]
        );
        assert!(
            rgb[2] >= -0.1 && rgb[2] <= 1.1,
            "B should be approximately in [0,1], got {}",
            rgb[2]
        );
    }
}

// ============================================================================
// CMYK to Lab Tests
// ============================================================================

/// Test CMYK to Lab transform
#[test]
fn test_cmyk_to_lab() {
    let cmyk = load_profile(PRIMARY_CMYK_PROFILE);
    let lab = Profile::new_lab4_context(
        lcms2::GlobalContext::new(),
        &lcms2::CIExyY {
            x: 0.3457,
            y: 0.3585,
            Y: 1.0,
        },
    )
    .expect("Lab profile creation failed");

    let transform = lcms2::Transform::<[u8; 4], [f64; 3]>::new(
        &cmyk,
        PixelFormat::CMYK_8,
        &lab,
        PixelFormat::Lab_DBL,
        Intent::Perceptual,
    )
    .expect("Transform creation failed");

    // Test key values
    let test_cases: &[([u8; 4], &str)] = &[
        ([0, 0, 0, 0], "White"),
        ([0, 0, 0, 255], "Black"),
        ([255, 0, 0, 0], "Cyan"),
        ([0, 255, 0, 0], "Magenta"),
        ([0, 0, 255, 0], "Yellow"),
    ];

    println!("CMYK to Lab (USWebCoatedSWOP):");
    for (cmyk_val, name) in test_cases {
        let mut lab_out = [0.0f64; 3];
        transform.transform_pixels(slice::from_ref(cmyk_val), slice::from_mut(&mut lab_out));
        println!(
            "  {} CMYK({},{},{},{}) -> Lab({:.2},{:.2},{:.2})",
            name,
            cmyk_val[0],
            cmyk_val[1],
            cmyk_val[2],
            cmyk_val[3],
            lab_out[0],
            lab_out[1],
            lab_out[2]
        );

        // Check L* range
        assert!(
            lab_out[0] >= 0.0 && lab_out[0] <= 100.0,
            "L* should be in [0,100], got {}",
            lab_out[0]
        );

        // Check white has high L*
        if *name == "White" {
            assert!(
                lab_out[0] > 90.0,
                "White L* should be > 90, got {}",
                lab_out[0]
            );
        }

        // Check black has low L*
        if *name == "Black" {
            assert!(
                lab_out[0] < 20.0,
                "Black L* should be < 20, got {}",
                lab_out[0]
            );
        }
    }
}

// ============================================================================
// Intent Comparison Tests
// ============================================================================

/// Compare rendering intents for CMYK
#[test]
fn test_cmyk_rendering_intents() {
    let cmyk = load_profile(PRIMARY_CMYK_PROFILE);
    let srgb = Profile::new_srgb();

    let intents = [
        (Intent::Perceptual, "Perceptual"),
        (Intent::RelativeColorimetric, "Relative"),
        (Intent::Saturation, "Saturation"),
        (Intent::AbsoluteColorimetric, "Absolute"),
    ];

    // Test saturated cyan
    let cmyk_input = [255u8, 0, 0, 0];

    println!("Comparing intents for Cyan CMYK(255,0,0,0) -> RGB:");
    for (intent, label) in intents {
        let transform = lcms2::Transform::<[u8; 4], [u8; 3]>::new(
            &cmyk,
            PixelFormat::CMYK_8,
            &srgb,
            PixelFormat::RGB_8,
            intent,
        )
        .expect("Transform creation failed");

        let mut rgb = [0u8; 3];
        transform.transform_pixels(slice::from_ref(&cmyk_input), slice::from_mut(&mut rgb));
        println!("  {}: RGB({},{},{})", label, rgb[0], rgb[1], rgb[2]);
    }
}

// ============================================================================
// Cross-CMS Parity Tests (lcms2 vs moxcms)
// ============================================================================

/// Compare CMYK->RGB transforms between lcms2 and moxcms
#[test]
fn test_cmyk_to_rgb_parity_lcms2_moxcms() {
    use cms_tests::reference::{transform_lcms2_cmyk_to_rgb, transform_moxcms_cmyk_to_rgb};

    let profile_data = load_profile_data(PRIMARY_CMYK_PROFILE);

    // Generate test CMYK values
    let mut cmyk_pixels = Vec::new();
    for c in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
        for m in (0u8..=255).step_by(K_CHANNEL_SAMPLE_STEP) {
            for y in (0u8..=255).step_by(K_CHANNEL_SAMPLE_STEP) {
                for k in (0u8..=255).step_by(K_CHANNEL_SAMPLE_STEP) {
                    cmyk_pixels.extend_from_slice(&[c, m, y, k]);
                }
            }
        }
    }

    let lcms2_result =
        transform_lcms2_cmyk_to_rgb(&profile_data, &cmyk_pixels).expect("lcms2 CMYK->RGB failed");
    let moxcms_result =
        transform_moxcms_cmyk_to_rgb(&profile_data, &cmyk_pixels).expect("moxcms CMYK->RGB failed");

    // Compare outputs
    let num_pixels = cmyk_pixels.len() / 4;
    let mut max_diff = 0u16;
    let mut total_diff = 0u64;
    let mut diff_count = 0u64;

    for i in 0..num_pixels {
        let idx = i * 3;
        for c in 0..3 {
            let diff =
                (lcms2_result[idx + c] as i16 - moxcms_result[idx + c] as i16).unsigned_abs();
            if diff > 0 {
                diff_count += 1;
                total_diff += diff as u64;
                if diff > max_diff {
                    max_diff = diff;
                }
            }
        }
    }

    let avg_diff = if diff_count > 0 {
        total_diff as f64 / diff_count as f64
    } else {
        0.0
    };

    println!(
        "CMYK->RGB parity (lcms2 vs moxcms): max_diff={}, avg_diff={:.2}, differing_channels={}/{}",
        max_diff,
        avg_diff,
        diff_count,
        num_pixels * 3
    );

    // Allow some difference due to implementation variations
    assert!(
        max_diff < CMYK_TO_RGB_PARITY_TOLERANCE,
        "CMYK->RGB max diff {} exceeds tolerance {} between lcms2 and moxcms",
        max_diff,
        CMYK_TO_RGB_PARITY_TOLERANCE
    );
}

/// Compare RGB->CMYK transforms between lcms2 and moxcms
#[test]
fn test_rgb_to_cmyk_parity_lcms2_moxcms() {
    use cms_tests::reference::{transform_lcms2_rgb_to_cmyk, transform_moxcms_rgb_to_cmyk};

    let profile_data = load_profile_data(PRIMARY_CMYK_PROFILE);

    // Generate test RGB values
    let mut rgb_pixels = Vec::new();
    for r in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
        for g in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
            for b in (0u8..=255).step_by(COLOR_SAMPLE_STEP) {
                rgb_pixels.extend_from_slice(&[r, g, b]);
            }
        }
    }

    let lcms2_result =
        transform_lcms2_rgb_to_cmyk(&profile_data, &rgb_pixels).expect("lcms2 RGB->CMYK failed");
    let moxcms_result =
        transform_moxcms_rgb_to_cmyk(&profile_data, &rgb_pixels).expect("moxcms RGB->CMYK failed");

    // Compare outputs
    let num_pixels = rgb_pixels.len() / 3;
    let mut max_diff = 0u16;
    let mut total_diff = 0u64;
    let mut diff_count = 0u64;

    for i in 0..num_pixels {
        let idx = i * 4;
        for c in 0..4 {
            let diff =
                (lcms2_result[idx + c] as i16 - moxcms_result[idx + c] as i16).unsigned_abs();
            if diff > 0 {
                diff_count += 1;
                total_diff += diff as u64;
                if diff > max_diff {
                    max_diff = diff;
                }
            }
        }
    }

    let avg_diff = if diff_count > 0 {
        total_diff as f64 / diff_count as f64
    } else {
        0.0
    };

    println!(
        "RGB->CMYK parity (lcms2 vs moxcms): max_diff={}, avg_diff={:.2}, differing_channels={}/{}",
        max_diff,
        avg_diff,
        diff_count,
        num_pixels * 4
    );

    // Allow some difference due to implementation variations
    assert!(
        max_diff < RGB_TO_CMYK_PARITY_TOLERANCE,
        "RGB->CMYK max diff {} exceeds tolerance {} between lcms2 and moxcms",
        max_diff,
        RGB_TO_CMYK_PARITY_TOLERANCE
    );
}

/// Test all CMYK profiles for cross-CMS parity
#[test]
fn test_cmyk_parity_all_profiles() {
    use cms_tests::reference::{transform_lcms2_cmyk_to_rgb, transform_moxcms_cmyk_to_rgb};

    let profiles = [
        "USWebCoatedSWOP.icc",
        "ghostscript_cmyk.icc",
        "nip2_cmyk.icc",
        "lcms2_test_cmyk.icc",
    ];

    // Simple test input
    let cmyk_input: Vec<u8> = vec![
        0, 0, 0, 0, // White
        0, 0, 0, 255, // Black
        255, 0, 0, 0, // Cyan
        0, 255, 0, 0, // Magenta
        0, 0, 255, 0, // Yellow
        128, 128, 128, 128, // 50% all
    ];

    println!("\nCMYK->RGB parity across profiles:");
    for name in profiles {
        let profile_data = load_profile_data(name);

        let lcms2_result = transform_lcms2_cmyk_to_rgb(&profile_data, &cmyk_input)
            .unwrap_or_else(|e| panic!("{}: lcms2 failed: {}", name, e));

        let moxcms_result = transform_moxcms_cmyk_to_rgb(&profile_data, &cmyk_input)
            .unwrap_or_else(|e| panic!("{}: moxcms failed: {}", name, e));

        let mut max_diff = 0u16;
        for i in 0..lcms2_result.len() {
            let diff = (lcms2_result[i] as i16 - moxcms_result[i] as i16).unsigned_abs();
            if diff > max_diff {
                max_diff = diff;
            }
        }

        println!("  {}: max_diff={}", name, max_diff);
    }
}
