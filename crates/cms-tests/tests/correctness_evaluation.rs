//! Comprehensive correctness evaluation for color management systems
//!
//! Uses lcms2 as the gold standard reference implementation.
//! Compares qcms, moxcms, and skcms against lcms2 for:
//! - Profile parsing success
//! - Transform output accuracy
//! - Edge case handling

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
                } else if path.extension().is_some_and(|e| e == "icc" || e == "icm") {
                    profiles.push(path);
                }
            }
        }
    }

    walk_dir(&profiles_dir, &mut profiles);

    // Also check images/compact-icc for minimal profiles
    let compact_dir = testdata_dir().join("images").join("compact-icc");
    walk_dir(&compact_dir, &mut profiles);

    // Also check images/skia for embedded profiles
    let skia_dir = testdata_dir().join("images").join("skia");
    walk_dir(&skia_dir, &mut profiles);

    profiles.sort();
    profiles
}

/// Standard test colors covering the full gamut
const TEST_COLORS_RGB8: &[[u8; 3]] = &[
    // Primaries
    [255, 0, 0], // Red
    [0, 255, 0], // Green
    [0, 0, 255], // Blue
    // Secondaries
    [255, 255, 0], // Yellow
    [255, 0, 255], // Magenta
    [0, 255, 255], // Cyan
    // Neutrals
    [0, 0, 0],       // Black
    [128, 128, 128], // Mid gray
    [255, 255, 255], // White
    // Skin tones
    [255, 224, 189], // Light skin
    [141, 85, 36],   // Dark skin
    // Pastels
    [255, 182, 193], // Pink
    [144, 238, 144], // Light green
    [173, 216, 230], // Light blue
    // Deep colors
    [128, 0, 0], // Maroon
    [0, 128, 0], // Dark green
    [0, 0, 128], // Navy
    // Grays
    [32, 32, 32],
    [64, 64, 64],
    [96, 96, 96],
    [160, 160, 160],
    [192, 192, 192],
    [224, 224, 224],
    // Random representative colors
    [210, 105, 30],  // Chocolate
    [255, 140, 0],   // Dark orange
    [75, 0, 130],    // Indigo
    [238, 130, 238], // Violet
    [0, 128, 128],   // Teal
];

/// Result of a transform comparison
#[derive(Debug)]
#[allow(dead_code)]
struct TransformResult {
    profile_name: String,
    lcms2_output: Vec<[u8; 3]>,
    qcms_output: Option<Vec<[u8; 3]>>,
    moxcms_output: Option<Vec<[u8; 3]>>,
    skcms_output: Option<Vec<[u8; 3]>>,
    qcms_max_diff: Option<i32>,
    moxcms_max_diff: Option<i32>,
    skcms_max_diff: Option<i32>,
}

/// Parse result for a profile
#[derive(Debug, Clone)]
struct ParseResult {
    lcms2: bool,
    qcms: bool,
    moxcms: bool,
    skcms: bool,
    moxcms_error: Option<String>,
}

/// Evaluate profile parsing across all CMS
fn evaluate_parsing(profiles: &[PathBuf]) -> HashMap<String, ParseResult> {
    let mut results = HashMap::new();

    for profile_path in profiles {
        let filename = profile_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let data = match std::fs::read(profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let lcms2_ok = lcms2::Profile::new_icc(&data).is_ok();
        let qcms_ok = qcms::Profile::new_from_slice(&data, false).is_some();
        let moxcms_result = moxcms::ColorProfile::new_from_slice(&data);
        let moxcms_ok = moxcms_result.is_ok();
        let moxcms_error = moxcms_result.err().map(|e| format!("{:?}", e));
        let skcms_ok = skcms_sys::parse_icc_profile(&data).is_some();

        results.insert(
            filename,
            ParseResult {
                lcms2: lcms2_ok,
                qcms: qcms_ok,
                moxcms: moxcms_ok,
                skcms: skcms_ok,
                moxcms_error,
            },
        );
    }

    results
}

/// Evaluate transform accuracy using lcms2 as reference
fn evaluate_transforms(profiles: &[PathBuf]) -> Vec<TransformResult> {
    let mut results = Vec::new();

    // Reference sRGB profiles
    let srgb_lcms2 = lcms2::Profile::new_srgb();
    let srgb_qcms = qcms::Profile::new_sRGB();
    let srgb_moxcms = moxcms::ColorProfile::new_srgb();
    let srgb_skcms = skcms_sys::srgb_profile();

    for profile_path in profiles {
        let filename = profile_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Skip known problematic profiles
        if filename.contains("bad")
            || filename.contains("toosmall")
            || filename.contains("CMYK")
            || filename.contains("cmyk")
            || filename.contains("Gray")
            || filename.contains("gray")
            || filename.contains("fuzz")
        {
            continue;
        }

        let data = match std::fs::read(profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Parse with lcms2 (reference)
        let lcms2_profile = match lcms2::Profile::new_icc(&data) {
            Ok(p) => p,
            Err(_) => continue, // Skip if lcms2 can't parse
        };

        // Create lcms2 transform: profile -> sRGB
        let lcms2_transform = match lcms2::Transform::new(
            &lcms2_profile,
            lcms2::PixelFormat::RGB_8,
            &srgb_lcms2,
            lcms2::PixelFormat::RGB_8,
            lcms2::Intent::Perceptual,
        ) {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Get reference outputs
        let mut lcms2_output = Vec::with_capacity(TEST_COLORS_RGB8.len());
        for color in TEST_COLORS_RGB8 {
            let mut out = [0u8; 3];
            lcms2_transform.transform_pixels(color, &mut out);
            lcms2_output.push(out);
        }

        // Try qcms
        let (qcms_output, qcms_max_diff) =
            if let Some(qcms_profile) = qcms::Profile::new_from_slice(&data, false) {
                if let Some(transform) = qcms::Transform::new(
                    &qcms_profile,
                    &srgb_qcms,
                    qcms::DataType::RGB8,
                    qcms::Intent::Perceptual,
                ) {
                    let mut output = Vec::with_capacity(TEST_COLORS_RGB8.len());
                    let mut max_diff = 0i32;

                    for (i, color) in TEST_COLORS_RGB8.iter().enumerate() {
                        let mut data = color.to_vec();
                        transform.apply(&mut data);
                        let out = [data[0], data[1], data[2]];

                        // Compare to lcms2
                        for c in 0..3 {
                            let diff = (out[c] as i32 - lcms2_output[i][c] as i32).abs();
                            max_diff = max_diff.max(diff);
                        }

                        output.push(out);
                    }

                    (Some(output), Some(max_diff))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        // Try moxcms
        let (moxcms_output, moxcms_max_diff) =
            if let Ok(moxcms_profile) = moxcms::ColorProfile::new_from_slice(&data) {
                if let Ok(transform) = moxcms_profile.create_transform_8bit(
                    moxcms::Layout::Rgb,
                    &srgb_moxcms,
                    moxcms::Layout::Rgb,
                    moxcms::TransformOptions::default(),
                ) {
                    let mut output = Vec::with_capacity(TEST_COLORS_RGB8.len());
                    let mut max_diff = 0i32;

                    for (i, color) in TEST_COLORS_RGB8.iter().enumerate() {
                        let mut out = [0u8; 3];
                        if transform.transform(color, &mut out).is_ok() {
                            // Compare to lcms2
                            for c in 0..3 {
                                let diff = (out[c] as i32 - lcms2_output[i][c] as i32).abs();
                                max_diff = max_diff.max(diff);
                            }
                            output.push(out);
                        } else {
                            output.push([0, 0, 0]);
                        }
                    }

                    (Some(output), Some(max_diff))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        // Try skcms
        let (skcms_output, skcms_max_diff) =
            if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
                let mut output = Vec::with_capacity(TEST_COLORS_RGB8.len());
                let mut max_diff = 0i32;
                let mut all_ok = true;

                for (i, color) in TEST_COLORS_RGB8.iter().enumerate() {
                    let mut out = [0u8; 3];
                    let ok = skcms_sys::transform(
                        color,
                        skcms_sys::skcms_PixelFormat::RGB_888,
                        skcms_sys::skcms_AlphaFormat::Opaque,
                        &skcms_profile,
                        &mut out,
                        skcms_sys::skcms_PixelFormat::RGB_888,
                        skcms_sys::skcms_AlphaFormat::Opaque,
                        srgb_skcms,
                        1,
                    );

                    if ok {
                        // Compare to lcms2
                        for c in 0..3 {
                            let diff = (out[c] as i32 - lcms2_output[i][c] as i32).abs();
                            max_diff = max_diff.max(diff);
                        }
                        output.push(out);
                    } else {
                        all_ok = false;
                        output.push([0, 0, 0]);
                    }
                }

                if all_ok {
                    (Some(output), Some(max_diff))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        results.push(TransformResult {
            profile_name: filename,
            lcms2_output,
            qcms_output,
            moxcms_output,
            skcms_output,
            qcms_max_diff,
            moxcms_max_diff,
            skcms_max_diff,
        });
    }

    results
}

/// Full correctness evaluation
#[test]
fn test_full_correctness_evaluation() {
    let profiles = collect_profiles();
    eprintln!("\n========================================");
    eprintln!("CORRECTNESS EVALUATION REPORT");
    eprintln!("========================================\n");
    eprintln!("Total profiles found: {}\n", profiles.len());

    // Phase 1: Parsing evaluation
    eprintln!("--- PHASE 1: PROFILE PARSING ---\n");
    let parse_results = evaluate_parsing(&profiles);

    let mut lcms2_count = 0;
    let mut qcms_count = 0;
    let mut moxcms_count = 0;
    let mut skcms_count = 0;

    for result in parse_results.values() {
        if result.lcms2 {
            lcms2_count += 1;
        }
        if result.qcms {
            qcms_count += 1;
        }
        if result.moxcms {
            moxcms_count += 1;
        }
        if result.skcms {
            skcms_count += 1;
        }
    }

    eprintln!("Parsing Success Rates:");
    eprintln!(
        "  lcms2:  {}/{} ({:.1}%)",
        lcms2_count,
        profiles.len(),
        100.0 * lcms2_count as f64 / profiles.len() as f64
    );
    eprintln!(
        "  skcms:  {}/{} ({:.1}%)",
        skcms_count,
        profiles.len(),
        100.0 * skcms_count as f64 / profiles.len() as f64
    );
    eprintln!(
        "  qcms:   {}/{} ({:.1}%)",
        qcms_count,
        profiles.len(),
        100.0 * qcms_count as f64 / profiles.len() as f64
    );
    eprintln!(
        "  moxcms: {}/{} ({:.1}%)",
        moxcms_count,
        profiles.len(),
        100.0 * moxcms_count as f64 / profiles.len() as f64
    );

    // Profiles that lcms2 parses but moxcms doesn't
    eprintln!("\n--- MOXCMS PARSING FAILURES (lcms2 succeeds) ---\n");
    let mut moxcms_failures: Vec<_> = parse_results
        .iter()
        .filter(|(_, r)| r.lcms2 && !r.moxcms)
        .collect();
    moxcms_failures.sort_by_key(|(name, _)| name.as_str());

    for (name, result) in &moxcms_failures {
        eprintln!(
            "  {}: {}",
            name,
            result.moxcms_error.as_deref().unwrap_or("unknown")
        );
    }
    eprintln!("\n  Total: {} profiles\n", moxcms_failures.len());

    // Phase 2: Transform evaluation
    eprintln!("--- PHASE 2: TRANSFORM ACCURACY ---\n");
    let transform_results = evaluate_transforms(&profiles);

    let mut qcms_perfect = 0;
    let mut qcms_close = 0; // diff <= 2
    let mut qcms_acceptable = 0; // diff <= 5
    let mut qcms_large = 0;

    let mut moxcms_perfect = 0;
    let mut moxcms_close = 0;
    let mut moxcms_acceptable = 0;
    let mut moxcms_large = 0;

    let mut skcms_perfect = 0;
    let mut skcms_close = 0;
    let mut skcms_acceptable = 0;
    let mut skcms_large = 0;

    let mut large_diff_profiles: Vec<(&str, i32, &str)> = Vec::new();

    for result in &transform_results {
        if let Some(diff) = result.qcms_max_diff {
            if diff == 0 {
                qcms_perfect += 1;
            } else if diff <= 2 {
                qcms_close += 1;
            } else if diff <= 5 {
                qcms_acceptable += 1;
            } else {
                qcms_large += 1;
                large_diff_profiles.push((&result.profile_name, diff, "qcms"));
            }
        }

        if let Some(diff) = result.moxcms_max_diff {
            if diff == 0 {
                moxcms_perfect += 1;
            } else if diff <= 2 {
                moxcms_close += 1;
            } else if diff <= 5 {
                moxcms_acceptable += 1;
            } else {
                moxcms_large += 1;
                large_diff_profiles.push((&result.profile_name, diff, "moxcms"));
            }
        }

        if let Some(diff) = result.skcms_max_diff {
            if diff == 0 {
                skcms_perfect += 1;
            } else if diff <= 2 {
                skcms_close += 1;
            } else if diff <= 5 {
                skcms_acceptable += 1;
            } else {
                skcms_large += 1;
                large_diff_profiles.push((&result.profile_name, diff, "skcms"));
            }
        }
    }

    let tested = transform_results.len();
    eprintln!(
        "Transform Accuracy vs lcms2 ({} RGB profiles tested):\n",
        tested
    );

    eprintln!("  qcms:");
    eprintln!("    Perfect (diff=0):     {}", qcms_perfect);
    eprintln!("    Close (diff<=2):      {}", qcms_close);
    eprintln!("    Acceptable (diff<=5): {}", qcms_acceptable);
    eprintln!("    Large (diff>5):       {}", qcms_large);

    eprintln!("\n  moxcms:");
    eprintln!("    Perfect (diff=0):     {}", moxcms_perfect);
    eprintln!("    Close (diff<=2):      {}", moxcms_close);
    eprintln!("    Acceptable (diff<=5): {}", moxcms_acceptable);
    eprintln!("    Large (diff>5):       {}", moxcms_large);

    eprintln!("\n  skcms:");
    eprintln!("    Perfect (diff=0):     {}", skcms_perfect);
    eprintln!("    Close (diff<=2):      {}", skcms_close);
    eprintln!("    Acceptable (diff<=5): {}", skcms_acceptable);
    eprintln!("    Large (diff>5):       {}", skcms_large);

    if !large_diff_profiles.is_empty() {
        eprintln!("\n--- PROFILES WITH LARGE DIFFERENCES ---\n");
        large_diff_profiles.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, diff, cms) in large_diff_profiles.iter().take(20) {
            eprintln!("  {}: max_diff={} ({})", name, diff, cms);
        }
    }

    eprintln!("\n========================================");
    eprintln!("END OF REPORT");
    eprintln!("========================================\n");

    // Assertions for CI
    assert!(
        lcms2_count > profiles.len() * 80 / 100,
        "lcms2 should parse at least 80% of profiles"
    );
}

/// Detailed analysis of moxcms parsing failures
#[test]
fn test_moxcms_failure_analysis() {
    let profiles = collect_profiles();
    eprintln!("\n=== MOXCMS FAILURE ANALYSIS ===\n");

    let mut error_types: HashMap<String, Vec<String>> = HashMap::new();

    for profile_path in &profiles {
        let filename = profile_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Skip fuzz/bad profiles
        if filename.contains("fuzz") || filename.contains("bad") || filename.contains("toosmall") {
            continue;
        }

        let data = match std::fs::read(profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Check if lcms2 can parse it
        if lcms2::Profile::new_icc(&data).is_err() {
            continue; // Skip profiles lcms2 can't parse
        }

        // Try moxcms
        if let Err(e) = moxcms::ColorProfile::new_from_slice(&data) {
            let error_str = format!("{:?}", e);
            error_types.entry(error_str).or_default().push(filename);
        }
    }

    // Sort by frequency
    let mut sorted: Vec<_> = error_types.iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (error, files) in sorted {
        eprintln!("Error: {} ({} profiles)", error, files.len());
        for file in files.iter().take(5) {
            eprintln!("  - {}", file);
        }
        if files.len() > 5 {
            eprintln!("  ... and {} more", files.len() - 5);
        }
        eprintln!();
    }
}

/// Test sRGB round-trip accuracy
#[test]
fn test_srgb_roundtrip_accuracy() {
    eprintln!("\n=== sRGB ROUND-TRIP ACCURACY ===\n");

    let srgb_lcms2 = lcms2::Profile::new_srgb();
    let srgb_moxcms = moxcms::ColorProfile::new_srgb();
    let srgb_skcms = skcms_sys::srgb_profile();

    // lcms2 sRGB -> sRGB identity
    let lcms2_identity = lcms2::Transform::new(
        &srgb_lcms2,
        lcms2::PixelFormat::RGB_8,
        &srgb_lcms2,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .expect("lcms2 identity transform");

    // moxcms sRGB -> sRGB identity
    let moxcms_identity = srgb_moxcms
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb_moxcms,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("moxcms identity transform");

    let mut lcms2_max_diff = 0i32;
    let mut moxcms_max_diff = 0i32;
    let mut skcms_max_diff = 0i32;

    for color in TEST_COLORS_RGB8 {
        // lcms2
        let mut lcms2_out = [0u8; 3];
        lcms2_identity.transform_pixels(color, &mut lcms2_out);
        for c in 0..3 {
            let diff = (color[c] as i32 - lcms2_out[c] as i32).abs();
            lcms2_max_diff = lcms2_max_diff.max(diff);
        }

        // moxcms
        let mut moxcms_out = [0u8; 3];
        moxcms_identity.transform(color, &mut moxcms_out).unwrap();
        for c in 0..3 {
            let diff = (color[c] as i32 - moxcms_out[c] as i32).abs();
            moxcms_max_diff = moxcms_max_diff.max(diff);
        }

        // skcms
        let mut skcms_out = [0u8; 3];
        skcms_sys::transform(
            color,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            srgb_skcms,
            &mut skcms_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            srgb_skcms,
            1,
        );
        for c in 0..3 {
            let diff = (color[c] as i32 - skcms_out[c] as i32).abs();
            skcms_max_diff = skcms_max_diff.max(diff);
        }
    }

    eprintln!("sRGB Identity Transform Max Diff:");
    eprintln!("  lcms2:  {}", lcms2_max_diff);
    eprintln!("  moxcms: {}", moxcms_max_diff);
    eprintln!("  skcms:  {}", skcms_max_diff);

    assert_eq!(lcms2_max_diff, 0, "lcms2 sRGB identity should be perfect");
    assert_eq!(moxcms_max_diff, 0, "moxcms sRGB identity should be perfect");
    assert_eq!(skcms_max_diff, 0, "skcms sRGB identity should be perfect");
}
