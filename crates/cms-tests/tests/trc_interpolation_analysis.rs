//! TRC (Tone Reproduction Curve) Interpolation Analysis
//!
//! This test suite investigates profiles where moxcms differs from browser
//! consensus (skcms/qcms) to understand TRC curve handling differences.
//!
//! Flagged profiles from previous analysis:
//! - alltags.icc - test profile with extreme values
//! - test3.icc, test4.icc - lcms2 test profiles
//! - sRGB_v4_ICC_preference.icc - v4 LUT profile
//! - BenQ_GL2450.icc, SM245B.icc - monitor profiles with large TRC curves
//! - Apple_Wide_Color.icc - device profile
//! - Kodak_sRGB.icc, Lexmark_X110.icc - device profiles

use skcms_sys::{skcms_AlphaFormat, skcms_PixelFormat};
use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Profiles flagged for TRC interpolation differences
const FLAGGED_PROFILES: &[&str] = &[
    "lcms2/fuzzers/alltags.icc",
    "lcms2/test3.icc",
    "lcms2/test4.icc",
    "skcms/color.org/sRGB_v4_ICC_preference.icc",
    "skcms/misc/SM245B.icc",
    "skcms/misc/BenQ_GL2450.icc",
    "skcms/misc/Apple_Wide_Color.icc",
    "skcms/misc/Kodak_sRGB.icc",
    "skcms/misc/Lexmark_X110.icc",
    "skcms/misc/MartiMaria_browsertest_A2B.icc",
];

#[derive(Debug)]
struct TrcAnalysis {
    name: String,
    profile_type: String,
    trc_type: String,
    curve_points: usize,
    max_browser_diff: i32,
    max_moxcms_vs_browser: i32,
    worst_input: u8,
    details: Vec<String>,
}

/// Analyze TRC curve characteristics of a profile
fn analyze_profile_trc(path: &Path) -> Option<TrcAnalysis> {
    let data = std::fs::read(path).ok()?;
    let name = path.file_name()?.to_string_lossy().to_string();

    // Parse with all CMS
    let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).ok()?;
    let qcms_profile = qcms::Profile::new_from_slice(&data, false)?;
    let skcms_profile = skcms_sys::parse_icc_profile(&data)?;
    let lcms2_profile = lcms2::Profile::new_icc(&data).ok()?;

    // Profile type info
    let profile_type = format!("{:?}", moxcms_profile.profile_class);
    let is_matrix_shaper = moxcms_profile.is_matrix_shaper();
    let trc_type = if is_matrix_shaper {
        "matrix-shaper".to_string()
    } else {
        "LUT-based".to_string()
    };

    // Estimate TRC curve size from profile
    let curve_points = 0; // Would need to inspect internals

    // Create transforms to sRGB
    let moxcms_srgb = moxcms::ColorProfile::new_srgb();
    let moxcms_transform = moxcms_profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .ok()?;

    let qcms_srgb = qcms::Profile::new_sRGB();
    let qcms_transform = qcms::Transform::new(
        &qcms_profile,
        &qcms_srgb,
        qcms::DataType::RGB8,
        qcms::Intent::Perceptual,
    )?;

    let lcms2_srgb = lcms2::Profile::new_srgb();
    let lcms2_transform = lcms2::Transform::new(
        &lcms2_profile,
        lcms2::PixelFormat::RGB_8,
        &lcms2_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .ok()?;

    let skcms_srgb = skcms_sys::srgb_profile();

    // Test all input values on neutral axis
    let mut max_browser_diff = 0i32;
    let mut max_moxcms_vs_browser = 0i32;
    let mut worst_input = 0u8;
    let mut details = Vec::new();

    for v in 0..=255 {
        let color = [v, v, v];

        // qcms
        let mut qcms_out = color.to_vec();
        qcms_transform.apply(&mut qcms_out);

        // moxcms
        let mut moxcms_out = [0u8; 3];
        moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

        // lcms2
        let mut lcms2_out = [0u8; 3];
        lcms2_transform.transform_pixels(&color, &mut lcms2_out);

        // skcms
        let mut skcms_out = [0u8; 3];
        skcms_sys::transform(
            &color,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut skcms_out,
            skcms_PixelFormat::RGB_888,
            skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );

        // Calculate differences
        let browser_diff = (qcms_out[0] as i32 - skcms_out[0] as i32).abs();
        let browser_avg = ((qcms_out[0] as i32 + skcms_out[0] as i32) / 2) as u8;
        let moxcms_vs_browser = (moxcms_out[0] as i32 - browser_avg as i32).abs();

        if moxcms_vs_browser > max_moxcms_vs_browser {
            max_moxcms_vs_browser = moxcms_vs_browser;
            worst_input = v;
        }
        max_browser_diff = max_browser_diff.max(browser_diff);

        // Record significant differences
        if moxcms_vs_browser > 2 || browser_diff > 2 {
            details.push(format!(
                "  v={}: qcms={} skcms={} moxcms={} lcms2={} | browser_diff={} mox_vs_browser={}",
                v, qcms_out[0], skcms_out[0], moxcms_out[0], lcms2_out[0],
                browser_diff, moxcms_vs_browser
            ));
        }
    }

    Some(TrcAnalysis {
        name,
        profile_type,
        trc_type,
        curve_points,
        max_browser_diff,
        max_moxcms_vs_browser,
        worst_input,
        details,
    })
}

/// Test TRC interpolation for flagged profiles
#[test]
fn test_trc_interpolation_flagged_profiles() {
    eprintln!("\n=== TRC Interpolation Analysis for Flagged Profiles ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let mut results = Vec::new();

    for profile_rel in FLAGGED_PROFILES {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            eprintln!("SKIP: {} (not found)", profile_rel);
            continue;
        }

        if let Some(analysis) = analyze_profile_trc(&profile_path) {
            results.push(analysis);
        } else {
            eprintln!("SKIP: {} (analysis failed)", profile_rel);
        }
    }

    eprintln!("Analyzed {} profiles:\n", results.len());

    for result in &results {
        eprintln!("Profile: {}", result.name);
        eprintln!("  Type: {} ({})", result.profile_type, result.trc_type);
        eprintln!("  Max browser diff (qcms vs skcms): {}", result.max_browser_diff);
        eprintln!("  Max moxcms vs browser consensus: {}", result.max_moxcms_vs_browser);
        eprintln!("  Worst input value: {}", result.worst_input);

        if !result.details.is_empty() {
            eprintln!("  Significant differences (first 10):");
            for detail in result.details.iter().take(10) {
                eprintln!("{}", detail);
            }
            if result.details.len() > 10 {
                eprintln!("  ... and {} more", result.details.len() - 10);
            }
        }
        eprintln!();
    }

    // Categorize results
    let exact_match: Vec<_> = results.iter().filter(|r| r.max_moxcms_vs_browser == 0).collect();
    let acceptable: Vec<_> = results.iter().filter(|r| r.max_moxcms_vs_browser > 0 && r.max_moxcms_vs_browser <= 2).collect();
    let differs: Vec<_> = results.iter().filter(|r| r.max_moxcms_vs_browser > 2).collect();

    eprintln!("=== Summary ===");
    eprintln!("Exact match with browser: {} profiles", exact_match.len());
    eprintln!("Acceptable (diff ≤ 2): {} profiles", acceptable.len());
    eprintln!("Differs from browser: {} profiles", differs.len());

    if !differs.is_empty() {
        eprintln!("\nProfiles needing investigation:");
        for r in &differs {
            eprintln!("  {} - max diff {} at input {}", r.name, r.max_moxcms_vs_browser, r.worst_input);
        }
    }
}

/// Deep dive into a specific profile's TRC
#[test]
fn test_trc_deep_dive_monitor_profiles() {
    eprintln!("\n=== TRC Deep Dive: Monitor Profiles ===\n");

    let profiles_dir = testdata_dir().join("profiles");

    // Focus on monitor profiles which often have custom TRC curves
    let monitor_profiles = [
        "skcms/misc/SM245B.icc",
        "skcms/misc/BenQ_GL2450.icc",
        "skcms/misc/Apple_Wide_Color.icc",
    ];

    for profile_rel in &monitor_profiles {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            continue;
        }

        let data = match std::fs::read(&profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let moxcms_profile = match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };

        eprintln!("Profile: {}", profile_rel);
        eprintln!("  Color space: {:?}", moxcms_profile.color_space);
        eprintln!("  PCS: {:?}", moxcms_profile.pcs);
        eprintln!("  Profile class: {:?}", moxcms_profile.profile_class);
        eprintln!("  Version: {:?}", moxcms_profile.version());
        eprintln!("  Is matrix-shaper: {}", moxcms_profile.is_matrix_shaper());
        eprintln!("  White point: {:?}", moxcms_profile.white_point);

        // If matrix-shaper, examine the colorant matrix
        if moxcms_profile.is_matrix_shaper() {
            let matrix = moxcms_profile.colorant_matrix();
            eprintln!("  Colorant matrix:");
            eprintln!("    R: [{:.6}, {:.6}, {:.6}]", matrix.v[0][0], matrix.v[0][1], matrix.v[0][2]);
            eprintln!("    G: [{:.6}, {:.6}, {:.6}]", matrix.v[1][0], matrix.v[1][1], matrix.v[1][2]);
            eprintln!("    B: [{:.6}, {:.6}, {:.6}]", matrix.v[2][0], matrix.v[2][1], matrix.v[2][2]);
        }

        eprintln!();
    }
}

/// Analyze TRC curve types across all profiles
#[test]
fn test_trc_type_distribution() {
    eprintln!("\n=== TRC Type Distribution ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let mut matrix_shaper_count = 0;
    let mut lut_count = 0;
    let mut total = 0;

    let subdirs = ["lcms2", "qcms", "skcms/misc", "skcms/mobile", "skcms/unusual"];

    for subdir in &subdirs {
        let dir = profiles_dir.join(subdir);
        if !dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "icc").unwrap_or(false) {
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(profile) = moxcms::ColorProfile::new_from_slice(&data) {
                        total += 1;
                        if profile.is_matrix_shaper() {
                            matrix_shaper_count += 1;
                        } else {
                            lut_count += 1;
                        }
                    }
                }
            }
        }
    }

    eprintln!("Total profiles parsed by moxcms: {}", total);
    eprintln!("Matrix-shaper profiles: {} ({:.1}%)", matrix_shaper_count, 100.0 * matrix_shaper_count as f64 / total as f64);
    eprintln!("LUT-based profiles: {} ({:.1}%)", lut_count, 100.0 * lut_count as f64 / total as f64);
    eprintln!();
    eprintln!("Note: TRC interpolation differences typically affect LUT-based profiles more than matrix-shaper profiles.");
}

/// Test specific dark color handling where differences often occur
#[test]
fn test_trc_dark_color_handling() {
    eprintln!("\n=== TRC Dark Color Handling ===\n");

    let profiles_dir = testdata_dir().join("profiles");

    // Dark colors (0-32) often show the most TRC interpolation differences
    let dark_values: Vec<u8> = (0..=32).collect();

    let test_profiles = [
        "skcms/color.org/sRGB_v4_ICC_preference.icc",
        "skcms/misc/SM245B.icc",
        "skcms/misc/MartiMaria_browsertest_A2B.icc",
    ];

    for profile_rel in &test_profiles {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            continue;
        }

        let data = match std::fs::read(&profile_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let moxcms_profile = match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let qcms_profile = match qcms::Profile::new_from_slice(&data, false) {
            Some(p) => p,
            None => continue,
        };

        let skcms_profile = match skcms_sys::parse_icc_profile(&data) {
            Some(p) => p,
            None => continue,
        };

        eprintln!("Profile: {}", profile_rel);
        eprintln!("Dark color TRC evaluation (input -> output):");

        // Create transforms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = match moxcms_profile.create_transform_8bit(
            moxcms::Layout::Rgb,
            &moxcms_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        ) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let qcms_srgb = qcms::Profile::new_sRGB();
        let qcms_transform = match qcms::Transform::new(
            &qcms_profile,
            &qcms_srgb,
            qcms::DataType::RGB8,
            qcms::Intent::Perceptual,
        ) {
            Some(t) => t,
            None => continue,
        };

        let skcms_srgb = skcms_sys::srgb_profile();

        let mut diffs = Vec::new();
        for &v in &dark_values {
            let color = [v, v, v];

            let mut qcms_out = color.to_vec();
            qcms_transform.apply(&mut qcms_out);

            let mut moxcms_out = [0u8; 3];
            moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

            let mut skcms_out = [0u8; 3];
            skcms_sys::transform(
                &color,
                skcms_PixelFormat::RGB_888,
                skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut skcms_out,
                skcms_PixelFormat::RGB_888,
                skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );

            let browser_avg = ((qcms_out[0] as i32 + skcms_out[0] as i32) / 2) as u8;
            let mox_vs_browser = (moxcms_out[0] as i32 - browser_avg as i32).abs();

            if mox_vs_browser > 0 {
                diffs.push((v, qcms_out[0], skcms_out[0], moxcms_out[0], mox_vs_browser));
            }
        }

        if diffs.is_empty() {
            eprintln!("  All dark colors match browser consensus");
        } else {
            eprintln!("  Input | qcms | skcms | moxcms | diff");
            eprintln!("  ------|------|-------|--------|-----");
            for (input, qcms, skcms, moxcms, diff) in &diffs {
                eprintln!("  {:5} | {:4} | {:5} | {:6} | {:4}", input, qcms, skcms, moxcms, diff);
            }
        }
        eprintln!();
    }
}
