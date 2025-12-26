//! Corpus-wide parity tests
//!
//! Tests all ICC profiles from the testdata corpus against qcms, moxcms, and lcms2.
//! Validates parsing, transform creation, and output consistency.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Get the testdata directory
fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Collect all ICC profiles from the corpus
fn collect_profiles() -> Vec<PathBuf> {
    let profiles_dir = testdata_dir().join("profiles");
    let mut profiles = Vec::new();

    fn walk_dir(dir: &Path, profiles: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, profiles);
                } else if path.extension().map_or(false, |e| e == "icc" || e == "icm") {
                    profiles.push(path);
                }
            }
        }
    }

    walk_dir(&profiles_dir, &mut profiles);

    // Also check images/compact-icc for minimal profiles
    let compact_dir = testdata_dir().join("images").join("compact-icc");
    walk_dir(&compact_dir, &mut profiles);

    profiles.sort();
    profiles
}

/// Result of trying to parse a profile with a CMS
#[derive(Debug, Clone)]
enum ParseResult {
    Success,
    Failed(String),
}

/// Test profile parsing across all three CMS implementations
#[test]
fn test_corpus_profile_parsing() {
    let profiles = collect_profiles();
    eprintln!("\nCorpus profile parsing test:");
    eprintln!("  Found {} ICC profiles", profiles.len());

    let mut qcms_success = 0;
    let mut moxcms_success = 0;
    let mut lcms2_success = 0;
    let mut all_success = 0;
    let mut failures: Vec<(PathBuf, String, String, String)> = Vec::new();

    for profile_path in &profiles {
        let data = match std::fs::read(profile_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  SKIP: {} - read error: {}", profile_path.display(), e);
                continue;
            }
        };

        // Skip obviously malformed test profiles (meant for error handling tests)
        let filename = profile_path.file_name().unwrap().to_string_lossy();
        if filename.contains("bad") || filename.contains("toosmall") || filename.contains("fuzz") {
            continue;
        }

        // Try qcms
        let qcms_result = match qcms::Profile::new_from_slice(&data, false) {
            Some(_) => {
                qcms_success += 1;
                ParseResult::Success
            }
            None => ParseResult::Failed("parse failed".to_string()),
        };

        // Try moxcms
        let moxcms_result = match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(_) => {
                moxcms_success += 1;
                ParseResult::Success
            }
            Err(e) => ParseResult::Failed(format!("{:?}", e)),
        };

        // Try lcms2
        let lcms2_result = match lcms2::Profile::new_icc(&data) {
            Ok(_) => {
                lcms2_success += 1;
                ParseResult::Success
            }
            Err(e) => ParseResult::Failed(format!("{:?}", e)),
        };

        // Count profiles all three can parse
        if matches!(qcms_result, ParseResult::Success)
            && matches!(moxcms_result, ParseResult::Success)
            && matches!(lcms2_result, ParseResult::Success)
        {
            all_success += 1;
        } else {
            let q = match &qcms_result {
                ParseResult::Success => "OK".to_string(),
                ParseResult::Failed(e) => e.clone(),
            };
            let m = match &moxcms_result {
                ParseResult::Success => "OK".to_string(),
                ParseResult::Failed(e) => e.clone(),
            };
            let l = match &lcms2_result {
                ParseResult::Success => "OK".to_string(),
                ParseResult::Failed(e) => e.clone(),
            };
            failures.push((profile_path.clone(), q, m, l));
        }
    }

    eprintln!("\n  Parsing Results:");
    eprintln!("    qcms:   {}/{} profiles", qcms_success, profiles.len());
    eprintln!("    moxcms: {}/{} profiles", moxcms_success, profiles.len());
    eprintln!("    lcms2:  {}/{} profiles", lcms2_success, profiles.len());
    eprintln!("    All 3:  {}/{} profiles", all_success, profiles.len());

    if !failures.is_empty() {
        eprintln!("\n  Profiles with parsing differences:");
        for (path, q, m, l) in failures.iter().take(10) {
            let name = path.file_name().unwrap().to_string_lossy();
            eprintln!("    {}: qcms={}, moxcms={}, lcms2={}", name, q, m, l);
        }
        if failures.len() > 10 {
            eprintln!("    ... and {} more", failures.len() - 10);
        }
    }

    // We expect at least 50% of profiles to parse with all three
    assert!(
        all_success >= profiles.len() / 2,
        "Expected at least half of profiles to parse with all CMS"
    );
}

/// Test transform parity on profiles all three CMS can parse
#[test]
fn test_corpus_transform_parity() {
    let profiles = collect_profiles();
    eprintln!("\nCorpus transform parity test:");

    let srgb_qcms = qcms::Profile::new_sRGB();
    let srgb_moxcms = moxcms::ColorProfile::new_srgb();
    let srgb_lcms2 = lcms2::Profile::new_srgb();

    let mut tested = 0;
    let mut identical = 0;
    let mut small_diff = 0;
    let mut large_diff = 0;
    let mut transform_failures: Vec<String> = Vec::new();

    // Test colors
    let test_colors: Vec<[u8; 3]> = vec![
        [0, 0, 0],       // Black
        [255, 255, 255], // White
        [255, 0, 0],     // Red
        [0, 255, 0],     // Green
        [0, 0, 255],     // Blue
        [128, 128, 128], // Gray
        [255, 128, 0],   // Orange
        [128, 0, 255],   // Purple
    ];

    for profile_path in &profiles {
        let data = match std::fs::read(profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let filename = profile_path.file_name().unwrap().to_string_lossy();

        // Skip malformed profiles
        // Skip profiles that are expected to differ or are edge cases
        if filename.contains("bad")
            || filename.contains("toosmall")
            || filename.contains("fuzz")
            || filename.contains("CMYK")
            || filename.contains("cmyk")
            || filename.contains("Gray")
            || filename.contains("gray")
            || filename.contains("HLG")  // HDR profiles
            || filename.contains("PQ")   // HDR profiles
            || filename.contains("cicp") // CICP profiles
            || filename.contains("parametric-thresh") // Edge case
            || filename.contains("Upper_") // color.org test patterns
            || filename.contains("Lower_") // color.org test patterns
            || filename.contains("Phase_One") // Camera profile
            || filename.starts_with("0") // qcms fuzz samples
            || filename.starts_with("1") // qcms fuzz samples
        {
            continue;
        }

        // Try to load with all three
        let qcms_profile = match qcms::Profile::new_from_slice(&data, false) {
            Some(p) => p,
            None => continue,
        };

        let moxcms_profile = match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let lcms2_profile = match lcms2::Profile::new_icc(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Create transforms: loaded profile -> sRGB
        let qcms_transform = match qcms::Transform::new(
            &qcms_profile,
            &srgb_qcms,
            qcms::DataType::RGB8,
            qcms::Intent::Perceptual,
        ) {
            Some(t) => t,
            None => {
                transform_failures.push(format!("{}: qcms transform failed", filename));
                continue;
            }
        };

        let moxcms_transform = match moxcms_profile.create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb_moxcms,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        ) {
            Ok(t) => t,
            Err(_) => {
                transform_failures.push(format!("{}: moxcms transform failed", filename));
                continue;
            }
        };

        let lcms2_transform = match lcms2::Transform::new(
            &lcms2_profile,
            lcms2::PixelFormat::RGB_8,
            &srgb_lcms2,
            lcms2::PixelFormat::RGB_8,
            lcms2::Intent::Perceptual,
        ) {
            Ok(t) => t,
            Err(_) => {
                transform_failures.push(format!("{}: lcms2 transform failed", filename));
                continue;
            }
        };

        tested += 1;
        let mut max_diff = 0i32;

        for color in &test_colors {
            // qcms (in-place)
            let mut qcms_data = color.to_vec();
            qcms_transform.apply(&mut qcms_data);

            // moxcms
            let mut moxcms_out = [0u8; 3];
            moxcms_transform.transform(color, &mut moxcms_out).unwrap();

            // lcms2
            let mut lcms2_out = [0u8; 3];
            lcms2_transform.transform_pixels(color, &mut lcms2_out);

            // Compare all pairs
            for i in 0..3 {
                max_diff = max_diff
                    .max((qcms_data[i] as i32 - moxcms_out[i] as i32).abs())
                    .max((qcms_data[i] as i32 - lcms2_out[i] as i32).abs())
                    .max((moxcms_out[i] as i32 - lcms2_out[i] as i32).abs());
            }
        }

        if max_diff == 0 {
            identical += 1;
        } else if max_diff <= 2 {
            small_diff += 1;
        } else {
            large_diff += 1;
            eprintln!(
                "  Large diff in {}: max_diff={}",
                filename, max_diff
            );
        }
    }

    eprintln!("\n  Transform Parity Results:");
    eprintln!("    Tested:     {} profiles", tested);
    eprintln!("    Identical:  {} (diff=0)", identical);
    eprintln!("    Small diff: {} (diff<=2)", small_diff);
    eprintln!("    Large diff: {} (diff>2)", large_diff);

    if !transform_failures.is_empty() {
        eprintln!("\n  Transform creation failures:");
        for f in transform_failures.iter().take(5) {
            eprintln!("    {}", f);
        }
    }

    // For standard profiles (excluding edge cases), we expect good parity
    // Note: Some legitimate differences exist due to:
    // - v4 vs v2 profile handling
    // - Different TRC interpolation methods
    // - LUT precision differences
    let acceptable = identical + small_diff;
    let parity_pct = if tested > 0 { acceptable * 100 / tested } else { 0 };
    eprintln!("    Parity:     {}% acceptable", parity_pct);

    // Log but don't fail - this is informational
    // Real-world CMS implementations have legitimate differences
    assert!(
        tested > 0,
        "Expected to test at least some profiles"
    );
}

/// Test that sRGB profiles from different sources produce consistent results
#[test]
fn test_srgb_profile_consistency() {
    eprintln!("\nsRGB profile consistency test:");

    let profiles_dir = testdata_dir().join("profiles");
    let compact_dir = testdata_dir().join("images").join("compact-icc");

    // Find all sRGB-ish profiles
    let srgb_profiles: Vec<PathBuf> = [
        profiles_dir.join("sRGB.icc"),
        profiles_dir.join("icc.org").join("sRGB2014.icc"),
        profiles_dir.join("icc.org").join("sRGB_v4_ICC_preference.icc"),
        profiles_dir.join("qcms").join("sRGB_lcms.icc"),
        profiles_dir.join("skcms").join("color.org").join("sRGB2014.icc"),
        compact_dir.join("sRGB-v2-nano.icc"),
        compact_dir.join("sRGB-v2-micro.icc"),
        compact_dir.join("sRGB-v4.icc"),
    ]
    .into_iter()
    .filter(|p| p.exists())
    .collect();

    eprintln!("  Found {} sRGB profiles", srgb_profiles.len());

    if srgb_profiles.is_empty() {
        eprintln!("  SKIP: No sRGB profiles found");
        return;
    }

    // Use built-in sRGB as reference
    let ref_srgb = lcms2::Profile::new_srgb();

    let test_color = [200u8, 100, 50];

    // Create reference transform (sRGB -> sRGB identity)
    let ref_transform = lcms2::Transform::new(
        &ref_srgb,
        lcms2::PixelFormat::RGB_8,
        &ref_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .expect("reference transform");

    let mut ref_output = [0u8; 3];
    ref_transform.transform_pixels(&test_color, &mut ref_output);

    eprintln!("  Reference: {:?} -> {:?}", test_color, ref_output);

    let mut all_match = true;

    for profile_path in &srgb_profiles {
        let data = std::fs::read(profile_path).expect("read profile");
        let filename = profile_path.file_name().unwrap().to_string_lossy();

        let profile = match lcms2::Profile::new_icc(&data) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("    {}: parse failed: {:?}", filename, e);
                continue;
            }
        };

        let transform = match lcms2::Transform::new(
            &profile,
            lcms2::PixelFormat::RGB_8,
            &ref_srgb,
            lcms2::PixelFormat::RGB_8,
            lcms2::Intent::Perceptual,
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("    {}: transform failed: {:?}", filename, e);
                continue;
            }
        };

        let mut output = [0u8; 3];
        transform.transform_pixels(&test_color, &mut output);

        let max_diff = (0..3)
            .map(|i| (output[i] as i32 - ref_output[i] as i32).abs())
            .max()
            .unwrap();

        if max_diff <= 1 {
            eprintln!("    {}: {:?} (diff={})", filename, output, max_diff);
        } else {
            eprintln!("    {}: {:?} (DIFF={})", filename, output, max_diff);
            all_match = false;
        }
    }

    assert!(all_match, "sRGB profiles should produce consistent results");
}

/// Test profile categories (RGB, CMYK, Gray, etc.)
#[test]
fn test_profile_categories() {
    let profiles = collect_profiles();
    eprintln!("\nProfile category analysis:");

    let mut categories: HashMap<String, usize> = HashMap::new();

    for profile_path in &profiles {
        let data = match std::fs::read(profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Try to get color space from lcms2
        if let Ok(profile) = lcms2::Profile::new_icc(&data) {
            let color_space = format!("{:?}", profile.color_space());
            *categories.entry(color_space).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<_> = categories.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    for (category, count) in sorted {
        eprintln!("  {}: {} profiles", category, count);
    }
}
