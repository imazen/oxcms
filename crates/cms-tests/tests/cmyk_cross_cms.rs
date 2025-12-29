//! Cross-CMS CMYK Comparison Tests
//!
//! This test module compares CMYK→sRGB conversions across different color management systems:
//! - moxcms (Rust)
//! - lcms2 (C, industry standard)
//! - skcms (C++, Chrome/Skia)
//!
//! ## Key Findings
//!
//! ### skcms CMYK Inversion (Photoshop Convention)
//!
//! skcms automatically inverts CMYK values before processing, assuming Photoshop's
//! "inverse CMYK" convention (see skcms.cc line 2820):
//!
//! ```text
//! // Photoshop creates CMYK images as inverse CMYK.
//! // These happen to be the only ones we've _ever_ seen.
//! add_op(Op::invert);
//! ```
//!
//! **Convention Differences:**
//!
//! | CMS | CMYK Convention | 0 = | 255 = |
//! |-----|-----------------|-----|-------|
//! | moxcms | ICC Standard | No ink (white) | Full ink |
//! | lcms2 | ICC Standard | No ink (white) | Full ink |
//! | skcms | Photoshop | Full ink | No ink (white) |
//!
//! ### Test Results (Coated_FOGRA39_CMYK.icc)
//!
//! **Grid Test (864 CMYK samples):**
//!
//! | Comparison | Max Diff | Avg Diff | >5: | >10: |
//! |------------|----------|----------|-----|------|
//! | moxcms vs lcms2 | 7 | 0.71 | 0% | 0% |
//! | skcms vs lcms2 | 255 | 145 | 100% | 100% |
//! | skcms(pre-inverted) vs lcms2 | 13 | 1.43 | 1% | 0% |
//!
//! The 255-point max difference with skcms is caused entirely by the convention mismatch.
//! When inputs are pre-inverted to match skcms's expectation, differences drop to ~1%.
//!
//! ### Impact on libjxl/JPEG XL
//!
//! When libjxl decodes a CMYK JXL image:
//! 1. CMYK data is stored in standard ICC convention (0 = no ink)
//! 2. skcms receives this data and inverts it (assuming Photoshop convention)
//! 3. Result: Colors are wrong - white becomes black, black becomes white
//!
//! **Issue #2**: The reported "CMYK→sRGB conversion produces different results than skcms/libjxl"
//! is not a bug in moxcms - it's a convention mismatch in how libjxl uses skcms.
//!
//! ### Recommendations
//!
//! For non-Photoshop CMYK sources (like JXL, TIFF, or ICC test profiles):
//! - Use lcms2 or moxcms which follow ICC standard convention
//! - Or pre-invert CMYK values (255-value) before passing to skcms

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// CMYK test value with human-readable name
#[derive(Debug, Clone)]
struct CmykTestValue {
    cmyk: [u8; 4],
    name: &'static str,
}

/// Results from transforming CMYK to sRGB
#[derive(Debug, Clone, Default)]
struct CmykToRgbResults {
    moxcms: Option<[u8; 3]>,
    lcms2: Option<[u8; 3]>,
    skcms: Option<[u8; 3]>,
    skcms_inverted_input: Option<[u8; 3]>, // What if we pre-invert input for skcms?
}

/// Calculate max channel difference between two RGB values
fn max_diff(a: &[u8; 3], b: &[u8; 3]) -> i32 {
    (0..3)
        .map(|i| (a[i] as i32 - b[i] as i32).abs())
        .max()
        .unwrap_or(0)
}

/// Standard CMYK test values (ICC convention: 0 = no ink, 255 = full ink)
fn get_test_values() -> Vec<CmykTestValue> {
    vec![
        CmykTestValue {
            cmyk: [0, 0, 0, 0],
            name: "White (no ink)",
        },
        CmykTestValue {
            cmyk: [0, 0, 0, 255],
            name: "Black (K only)",
        },
        CmykTestValue {
            cmyk: [255, 0, 0, 0],
            name: "Cyan 100%",
        },
        CmykTestValue {
            cmyk: [0, 255, 0, 0],
            name: "Magenta 100%",
        },
        CmykTestValue {
            cmyk: [0, 0, 255, 0],
            name: "Yellow 100%",
        },
        CmykTestValue {
            cmyk: [255, 255, 0, 0],
            name: "Cyan+Magenta (Blue)",
        },
        CmykTestValue {
            cmyk: [255, 0, 255, 0],
            name: "Cyan+Yellow (Green)",
        },
        CmykTestValue {
            cmyk: [0, 255, 255, 0],
            name: "Magenta+Yellow (Red)",
        },
        CmykTestValue {
            cmyk: [255, 255, 255, 255],
            name: "Rich Black (all 100%)",
        },
        CmykTestValue {
            cmyk: [128, 128, 128, 128],
            name: "50% all channels",
        },
        CmykTestValue {
            cmyk: [64, 64, 64, 64],
            name: "25% all channels",
        },
        CmykTestValue {
            cmyk: [192, 192, 192, 192],
            name: "75% all channels",
        },
    ]
}

/// Transform CMYK to RGB using moxcms
fn transform_moxcms(cmyk_profile_data: &[u8], cmyk: [u8; 4]) -> Option<[u8; 3]> {
    let cmyk_profile = moxcms::ColorProfile::new_from_slice(cmyk_profile_data).ok()?;
    let srgb_profile = moxcms::ColorProfile::new_srgb();

    // moxcms uses Layout::Rgba for 4-channel CMYK
    let transform = cmyk_profile
        .create_transform_8bit(
            moxcms::Layout::Rgba,
            &srgb_profile,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .ok()?;

    let mut rgb = [0u8; 3];
    transform.transform(&cmyk, &mut rgb).ok()?;
    Some(rgb)
}

/// Transform CMYK to RGB using lcms2
fn transform_lcms2(cmyk_profile_data: &[u8], cmyk: [u8; 4]) -> Option<[u8; 3]> {
    use std::slice;

    let cmyk_profile = lcms2::Profile::new_icc(cmyk_profile_data).ok()?;
    let srgb_profile = lcms2::Profile::new_srgb();

    let transform = lcms2::Transform::<[u8; 4], [u8; 3]>::new(
        &cmyk_profile,
        lcms2::PixelFormat::CMYK_8,
        &srgb_profile,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .ok()?;

    let mut rgb = [0u8; 3];
    // Use slice::from_ref/from_mut to properly pass references to the transform
    transform.transform_pixels(slice::from_ref(&cmyk), slice::from_mut(&mut rgb));
    Some(rgb)
}

/// Transform CMYK to RGB using skcms
///
/// Note: skcms uses RGBA_8888 format for CMYK (4 channels)
/// and automatically inverts CMYK values (Photoshop convention)
fn transform_skcms(cmyk_profile_data: &[u8], cmyk: [u8; 4]) -> Option<[u8; 3]> {
    let cmyk_profile = skcms_sys::parse_icc_profile(cmyk_profile_data)?;

    // Check that it's actually a CMYK profile
    if cmyk_profile.data_color_space != skcms_sys::skcms_Signature::CMYK as u32 {
        return None;
    }

    let srgb_profile = skcms_sys::srgb_profile();

    // skcms uses RGBA_8888 for 4-channel data (CMYK)
    let mut rgba_out = [0u8; 4];
    let success = skcms_sys::transform(
        &cmyk,
        skcms_sys::skcms_PixelFormat::RGBA_8888,
        skcms_sys::skcms_AlphaFormat::Unpremul,
        &cmyk_profile,
        &mut rgba_out,
        skcms_sys::skcms_PixelFormat::RGB_888,
        skcms_sys::skcms_AlphaFormat::Opaque,
        srgb_profile,
        1,
    );

    if success {
        Some([rgba_out[0], rgba_out[1], rgba_out[2]])
    } else {
        None
    }
}

/// Transform CMYK to RGB using skcms with pre-inverted input
///
/// This tests whether pre-inverting CMYK values makes skcms match lcms2/moxcms
fn transform_skcms_with_inverted_input(cmyk_profile_data: &[u8], cmyk: [u8; 4]) -> Option<[u8; 3]> {
    // Pre-invert the CMYK values (convert from ICC convention to Photoshop convention)
    let inverted_cmyk = [255 - cmyk[0], 255 - cmyk[1], 255 - cmyk[2], 255 - cmyk[3]];
    transform_skcms(cmyk_profile_data, inverted_cmyk)
}

/// Compare CMYK→sRGB across all CMS libraries
#[test]
fn test_cmyk_cross_cms_comparison() {
    eprintln!("\n=== Cross-CMS CMYK→sRGB Comparison ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let fogra39_path = profiles_dir.join("skcms/misc/Coated_FOGRA39_CMYK.icc");

    if !fogra39_path.exists() {
        eprintln!(
            "SKIP: Coated_FOGRA39_CMYK.icc not found at {:?}",
            fogra39_path
        );
        return;
    }

    let profile_data = std::fs::read(&fogra39_path).unwrap();
    eprintln!(
        "Using profile: Coated_FOGRA39_CMYK.icc ({} bytes)",
        profile_data.len()
    );

    // Verify profile can be loaded by each CMS
    let moxcms_ok = moxcms::ColorProfile::new_from_slice(&profile_data).is_ok();
    let lcms2_ok = lcms2::Profile::new_icc(&profile_data).is_ok();
    let skcms_ok = skcms_sys::parse_icc_profile(&profile_data).is_some();

    eprintln!(
        "Profile loading: moxcms={} lcms2={} skcms={}",
        if moxcms_ok { "✓" } else { "✗" },
        if lcms2_ok { "✓" } else { "✗" },
        if skcms_ok { "✓" } else { "✗" }
    );

    if !moxcms_ok || !lcms2_ok || !skcms_ok {
        eprintln!("SKIP: Not all CMS libraries could load the profile");
        return;
    }

    let test_values = get_test_values();

    eprintln!(
        "\n{:<25} {:>15} {:>15} {:>15} {:>15} {:>10} {:>10}",
        "Test Value", "moxcms", "lcms2", "skcms", "skcms(inv)", "mox-lcms", "skcms-lcms"
    );
    eprintln!("{}", "-".repeat(115));

    let mut max_mox_lcms = 0i32;
    let mut max_skcms_lcms = 0i32;
    let mut max_skcms_inv_lcms = 0i32;
    let mut results_list = Vec::new();

    for test in &test_values {
        let results = CmykToRgbResults {
            moxcms: transform_moxcms(&profile_data, test.cmyk),
            lcms2: transform_lcms2(&profile_data, test.cmyk),
            skcms: transform_skcms(&profile_data, test.cmyk),
            skcms_inverted_input: transform_skcms_with_inverted_input(&profile_data, test.cmyk),
        };

        // Calculate differences
        let mox_lcms = match (&results.moxcms, &results.lcms2) {
            (Some(m), Some(l)) => max_diff(m, l),
            _ => -1,
        };
        let skcms_lcms = match (&results.skcms, &results.lcms2) {
            (Some(s), Some(l)) => max_diff(s, l),
            _ => -1,
        };
        let skcms_inv_lcms = match (&results.skcms_inverted_input, &results.lcms2) {
            (Some(s), Some(l)) => max_diff(s, l),
            _ => -1,
        };

        if mox_lcms > max_mox_lcms {
            max_mox_lcms = mox_lcms;
        }
        if skcms_lcms > max_skcms_lcms {
            max_skcms_lcms = skcms_lcms;
        }
        if skcms_inv_lcms > max_skcms_inv_lcms {
            max_skcms_inv_lcms = skcms_inv_lcms;
        }

        // Format output
        let fmt_rgb = |rgb: &Option<[u8; 3]>| match rgb {
            Some([r, g, b]) => format!("{:3},{:3},{:3}", r, g, b),
            None => "  FAILED  ".to_string(),
        };

        eprintln!(
            "{:<25} {:>15} {:>15} {:>15} {:>15} {:>10} {:>10}",
            test.name,
            fmt_rgb(&results.moxcms),
            fmt_rgb(&results.lcms2),
            fmt_rgb(&results.skcms),
            fmt_rgb(&results.skcms_inverted_input),
            if mox_lcms >= 0 {
                format!("{}", mox_lcms)
            } else {
                "N/A".to_string()
            },
            if skcms_lcms >= 0 {
                format!("{}", skcms_lcms)
            } else {
                "N/A".to_string()
            },
        );

        results_list.push((test.clone(), results));
    }

    eprintln!("{}", "-".repeat(115));
    eprintln!("\nMaximum differences:");
    eprintln!("  moxcms vs lcms2: {}", max_mox_lcms);
    eprintln!("  skcms vs lcms2: {}", max_skcms_lcms);
    eprintln!("  skcms(inverted input) vs lcms2: {}", max_skcms_inv_lcms);

    eprintln!("\n=== Analysis ===");
    eprintln!("skcms automatically inverts CMYK values (Photoshop convention).");
    if max_skcms_inv_lcms < max_skcms_lcms {
        eprintln!(
            "Pre-inverting input reduces skcms/lcms2 difference from {} to {}",
            max_skcms_lcms, max_skcms_inv_lcms
        );
        eprintln!("This confirms skcms expects Photoshop's inverted CMYK convention.");
    }

    // Test with inverted convention grid
    eprintln!("\n=== Grid Test: Checking for systematic differences ===");
    test_cmyk_grid(&profile_data);
}

/// Test a grid of CMYK values to find systematic differences
fn test_cmyk_grid(profile_data: &[u8]) {
    let mut mox_lcms_diffs = Vec::new();
    let mut skcms_lcms_diffs = Vec::new();
    let mut skcms_inv_lcms_diffs = Vec::new();

    let step = 51; // 0, 51, 102, 153, 204, 255 = 6 values per channel
    let k_step = 85; // Coarser for K: 0, 85, 170, 255

    for c in (0..=255).step_by(step) {
        for m in (0..=255).step_by(step) {
            for y in (0..=255).step_by(step) {
                for k in (0..=255).step_by(k_step) {
                    let cmyk = [c as u8, m as u8, y as u8, k as u8];

                    let moxcms_rgb = transform_moxcms(profile_data, cmyk);
                    let lcms2_rgb = transform_lcms2(profile_data, cmyk);
                    let skcms_rgb = transform_skcms(profile_data, cmyk);
                    let skcms_inv_rgb = transform_skcms_with_inverted_input(profile_data, cmyk);

                    if let (Some(m), Some(l)) = (&moxcms_rgb, &lcms2_rgb) {
                        let diff = max_diff(m, l);
                        mox_lcms_diffs.push((cmyk, diff, *m, *l));
                    }

                    if let (Some(s), Some(l)) = (&skcms_rgb, &lcms2_rgb) {
                        let diff = max_diff(s, l);
                        skcms_lcms_diffs.push((cmyk, diff, *s, *l));
                    }

                    if let (Some(s), Some(l)) = (&skcms_inv_rgb, &lcms2_rgb) {
                        let diff = max_diff(s, l);
                        skcms_inv_lcms_diffs.push((cmyk, diff, *s, *l));
                    }
                }
            }
        }
    }

    // Report statistics
    let report_stats = |name: &str, diffs: &[(_, i32, _, _)]| {
        if diffs.is_empty() {
            eprintln!("  {}: No data", name);
            return;
        }
        let max = diffs.iter().map(|d| d.1).max().unwrap_or(0);
        let sum: i64 = diffs.iter().map(|d| d.1 as i64).sum();
        let avg = sum as f64 / diffs.len() as f64;
        let over_5 = diffs.iter().filter(|d| d.1 > 5).count();
        let over_10 = diffs.iter().filter(|d| d.1 > 10).count();

        eprintln!(
            "  {}: max={}, avg={:.2}, >5: {} ({}%), >10: {} ({}%)",
            name,
            max,
            avg,
            over_5,
            over_5 * 100 / diffs.len(),
            over_10,
            over_10 * 100 / diffs.len()
        );

        // Show worst cases
        let mut worst: Vec<_> = diffs.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(&a.1));
        if worst[0].1 > 10 {
            eprintln!("    Worst cases:");
            for (cmyk, diff, result, lcms2) in worst.iter().take(3) {
                eprintln!(
                    "      CMYK {:?} -> {:?} vs lcms2 {:?} (diff={})",
                    cmyk, result, lcms2, diff
                );
            }
        }
    };

    eprintln!("\nGrid test results ({} samples):", mox_lcms_diffs.len());
    report_stats("moxcms vs lcms2", &mox_lcms_diffs);
    report_stats("skcms vs lcms2", &skcms_lcms_diffs);
    report_stats("skcms(inv) vs lcms2", &skcms_inv_lcms_diffs);
}

/// Test that skcms CMYK inversion hypothesis is correct
#[test]
fn test_skcms_cmyk_inversion_hypothesis() {
    eprintln!("\n=== skcms CMYK Inversion Hypothesis Test ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let fogra39_path = profiles_dir.join("skcms/misc/Coated_FOGRA39_CMYK.icc");

    if !fogra39_path.exists() {
        eprintln!("SKIP: Profile not found");
        return;
    }

    let profile_data = std::fs::read(&fogra39_path).unwrap();

    // The hypothesis: skcms interprets CMYK values inverted (Photoshop convention)
    // So CMYK [0,0,0,0] in skcms means [255,255,255,255] in ICC convention (full coverage)
    // And CMYK [255,255,255,255] in skcms means [0,0,0,0] in ICC convention (white)

    eprintln!("Testing hypothesis that skcms uses inverted CMYK convention:");
    eprintln!("  - skcms [0,0,0,0] should map like lcms2 [255,255,255,255] (rich black)");
    eprintln!("  - skcms [255,255,255,255] should map like lcms2 [0,0,0,0] (white)\n");

    // Test case 1: skcms white = [255,255,255,255] should match lcms2 white = [0,0,0,0]
    let skcms_white = transform_skcms(&profile_data, [255, 255, 255, 255]);
    let lcms2_white = transform_lcms2(&profile_data, [0, 0, 0, 0]);

    eprintln!("Test 1: White");
    eprintln!("  skcms  [255,255,255,255] -> {:?}", skcms_white);
    eprintln!("  lcms2  [0,0,0,0]         -> {:?}", lcms2_white);
    if let (Some(s), Some(l)) = (&skcms_white, &lcms2_white) {
        eprintln!("  Difference: {}", max_diff(s, l));
    }

    // Test case 2: skcms black = [0,0,0,0] should match lcms2 rich black = [255,255,255,255]
    let skcms_black = transform_skcms(&profile_data, [0, 0, 0, 0]);
    let lcms2_black = transform_lcms2(&profile_data, [255, 255, 255, 255]);

    eprintln!("\nTest 2: Rich Black");
    eprintln!("  skcms  [0,0,0,0]         -> {:?}", skcms_black);
    eprintln!("  lcms2  [255,255,255,255] -> {:?}", lcms2_black);
    if let (Some(s), Some(l)) = (&skcms_black, &lcms2_black) {
        eprintln!("  Difference: {}", max_diff(s, l));
    }

    // Test case 3: Verify normal case without inversion gives large differences
    let skcms_normal_white = transform_skcms(&profile_data, [0, 0, 0, 0]);
    let lcms2_normal_white = transform_lcms2(&profile_data, [0, 0, 0, 0]);

    eprintln!("\nTest 3: Same input [0,0,0,0] (expecting large difference)");
    eprintln!("  skcms  [0,0,0,0] -> {:?}", skcms_normal_white);
    eprintln!("  lcms2  [0,0,0,0] -> {:?}", lcms2_normal_white);
    if let (Some(s), Some(l)) = (&skcms_normal_white, &lcms2_normal_white) {
        let diff = max_diff(s, l);
        eprintln!("  Difference: {}", diff);
        if diff > 100 {
            eprintln!("  ✓ Large difference confirms skcms uses inverted convention");
        }
    }
}

/// Test with multiple CMYK profiles
#[test]
fn test_cmyk_multiple_profiles() {
    eprintln!("\n=== Multi-Profile CMYK Comparison ===\n");

    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/icc");

    let profiles = [
        ("USWebCoatedSWOP.icc", "US Web Coated (SWOP) v2"),
        ("ghostscript_cmyk.icc", "Artifex CMYK SWOP"),
        ("nip2_cmyk.icc", "Chemical proof"),
        ("lcms2_test_cmyk.icc", "lcms2 testbed"),
    ];

    for (filename, description) in profiles {
        let path = fixtures_dir.join(filename);
        if !path.exists() {
            eprintln!("SKIP: {} not found", filename);
            continue;
        }

        let profile_data = std::fs::read(&path).unwrap();
        eprintln!("\n--- {} ({}) ---", filename, description);

        // Test white (no ink)
        let cmyk = [0u8, 0, 0, 0];
        let mox = transform_moxcms(&profile_data, cmyk);
        let lcms = transform_lcms2(&profile_data, cmyk);
        let skcms = transform_skcms(&profile_data, cmyk);

        eprintln!("  White [0,0,0,0]:");
        eprintln!("    moxcms: {:?}", mox);
        eprintln!("    lcms2:  {:?}", lcms);
        eprintln!("    skcms:  {:?}", skcms);

        if let (Some(m), Some(l)) = (&mox, &lcms) {
            eprintln!("    moxcms vs lcms2: {}", max_diff(m, l));
        }
        if let (Some(s), Some(l)) = (&skcms, &lcms) {
            eprintln!("    skcms vs lcms2: {}", max_diff(s, l));
        }

        // Test K only black
        let cmyk = [0u8, 0, 0, 255];
        let mox = transform_moxcms(&profile_data, cmyk);
        let lcms = transform_lcms2(&profile_data, cmyk);

        eprintln!("  K-only Black [0,0,0,255]:");
        eprintln!("    moxcms: {:?}", mox);
        eprintln!("    lcms2:  {:?}", lcms);

        if let (Some(m), Some(l)) = (&mox, &lcms) {
            let diff = max_diff(m, l);
            if diff > 5 {
                eprintln!("    ⚠ moxcms vs lcms2: {} (> 5)", diff);
            } else {
                eprintln!("    moxcms vs lcms2: {}", diff);
            }
        }
    }
}
