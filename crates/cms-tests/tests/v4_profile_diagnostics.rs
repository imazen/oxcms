//! Diagnostic test for ICC v4 profile transform differences
//!
//! Analyzes the parity differences for ICC v4 sRGB profiles between
//! moxcms and lcms2, and compares against browser consensus (skcms/qcms).
//!
//! These profiles show max_diff=11 in correctness tests:
//! - sRGB_ICC_v4_Appearance.icc
//! - sRGB_v4_ICC_preference.icc
//! - sRGB_ICC_v4_beta.icc

use std::path::Path;

/// Get the testdata directory
fn testdata_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Standard test colors covering the full gamut
const TEST_COLORS_RGB8: &[[u8; 3]] = &[
    // Primaries
    [255, 0, 0],     // Red
    [0, 255, 0],     // Green
    [0, 0, 255],     // Blue
    // Secondaries
    [255, 255, 0],   // Yellow
    [255, 0, 255],   // Magenta
    [0, 255, 255],   // Cyan
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
    [128, 0, 0],     // Maroon
    [0, 128, 0],     // Dark green
    [0, 0, 128],     // Navy
    // Grays
    [32, 32, 32],
    [64, 64, 64],
    [96, 96, 96],
    [160, 160, 160],
    [192, 192, 192],
    [224, 224, 224],
];

/// Result of comparing a single color across all CMS implementations
#[derive(Debug)]
#[allow(dead_code)]
struct ColorComparison {
    input: [u8; 3],
    lcms2: [u8; 3],
    moxcms: Option<[u8; 3]>,
    skcms: Option<[u8; 3]>,
    qcms: Option<[u8; 3]>,
    // Differences relative to lcms2
    moxcms_diff: Option<i32>,
    skcms_diff: Option<i32>,
    qcms_diff: Option<i32>,
    // Browser consensus (skcms and qcms agree)
    browser_consensus: Option<[u8; 3]>,
    browser_diff_from_lcms2: Option<i32>,
}

impl ColorComparison {
    /// Check which channel has the largest difference
    fn max_diff_channel(&self) -> Option<(usize, i32)> {
        if let Some(mox) = self.moxcms {
            let mut max_ch = 0;
            let mut max_diff = 0i32;
            for c in 0..3 {
                let diff = (mox[c] as i32 - self.lcms2[c] as i32).abs();
                if diff > max_diff {
                    max_diff = diff;
                    max_ch = c;
                }
            }
            if max_diff > 0 {
                return Some((max_ch, max_diff));
            }
        }
        None
    }

    /// Check if moxcms matches browser consensus better than lcms2
    fn moxcms_matches_browsers(&self) -> bool {
        if let (Some(mox), Some(consensus)) = (self.moxcms, self.browser_consensus) {
            let mox_browser_diff: i32 = (0..3)
                .map(|c| (mox[c] as i32 - consensus[c] as i32).abs())
                .max()
                .unwrap_or(0);

            let lcms2_browser_diff: i32 = (0..3)
                .map(|c| (self.lcms2[c] as i32 - consensus[c] as i32).abs())
                .max()
                .unwrap_or(0);

            mox_browser_diff < lcms2_browser_diff
        } else {
            false
        }
    }
}

/// Detailed profile analysis result
#[derive(Debug)]
#[allow(dead_code)]
struct ProfileAnalysis {
    profile_name: String,
    comparisons: Vec<ColorComparison>,
    max_moxcms_diff: i32,
    max_skcms_diff: i32,
    max_qcms_diff: i32,
    // How many times moxcms matches browser consensus better than lcms2
    moxcms_better_count: usize,
    // Channel statistics
    channel_diff_stats: [ChannelStats; 3],
}

#[derive(Debug, Clone)]
struct ChannelStats {
    channel_name: &'static str,
    max_diff: i32,
    diff_count: usize,
    total_diff: i64,
}

impl ChannelStats {
    fn new(channel_name: &'static str) -> Self {
        Self {
            channel_name,
            max_diff: 0,
            diff_count: 0,
            total_diff: 0,
        }
    }

    fn mean_diff(&self) -> f64 {
        if self.diff_count > 0 {
            self.total_diff as f64 / self.diff_count as f64
        } else {
            0.0
        }
    }
}

/// Analyze a single ICC profile across all CMS implementations
fn analyze_profile(profile_path: &Path) -> Option<ProfileAnalysis> {
    let filename = profile_path.file_name()?.to_string_lossy().to_string();

    let data = std::fs::read(profile_path).ok()?;

    // Reference sRGB profiles
    let srgb_lcms2 = lcms2::Profile::new_srgb();
    let srgb_moxcms = moxcms::ColorProfile::new_srgb();
    let srgb_skcms = skcms_sys::srgb_profile();
    let srgb_qcms = qcms::Profile::new_sRGB();

    // Parse profile with all CMS
    let lcms2_profile = lcms2::Profile::new_icc(&data).ok()?;
    let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).ok();
    let skcms_profile = skcms_sys::parse_icc_profile(&data);
    let qcms_profile = qcms::Profile::new_from_slice(&data, false);

    // Create transforms: profile -> sRGB
    let lcms2_transform = lcms2::Transform::new(
        &lcms2_profile,
        lcms2::PixelFormat::RGB_8,
        &srgb_lcms2,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .ok()?;

    let moxcms_transform = moxcms_profile.as_ref().and_then(|p| {
        p.create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb_moxcms,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .ok()
    });

    let skcms_profile_ref = skcms_profile.as_ref();
    let qcms_transform = qcms_profile.as_ref().and_then(|p| {
        qcms::Transform::new(p, &srgb_qcms, qcms::DataType::RGB8, qcms::Intent::Perceptual)
    });

    // Transform all test colors
    let mut comparisons = Vec::new();
    let mut max_moxcms_diff = 0i32;
    let mut max_skcms_diff = 0i32;
    let mut max_qcms_diff = 0i32;
    let mut moxcms_better_count = 0usize;
    let mut channel_stats = [
        ChannelStats::new("R"),
        ChannelStats::new("G"),
        ChannelStats::new("B"),
    ];

    for color in TEST_COLORS_RGB8 {
        // lcms2 (reference)
        let mut lcms2_out = [0u8; 3];
        lcms2_transform.transform_pixels(color, &mut lcms2_out);

        // moxcms
        let moxcms_out = moxcms_transform.as_ref().and_then(|t| {
            let mut out = [0u8; 3];
            t.transform(color, &mut out).ok().map(|_| out)
        });

        // skcms
        let skcms_out = skcms_profile_ref.map(|p| {
            let mut out = [0u8; 3];
            skcms_sys::transform(
                color,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                p,
                &mut out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                srgb_skcms,
                1,
            );
            out
        });

        // qcms
        let qcms_out = qcms_transform.as_ref().map(|t| {
            let mut data = color.to_vec();
            t.apply(&mut data);
            [data[0], data[1], data[2]]
        });

        // Calculate differences
        let moxcms_diff = moxcms_out.map(|mox| {
            let diff: i32 = (0..3)
                .map(|c| (mox[c] as i32 - lcms2_out[c] as i32).abs())
                .max()
                .unwrap();

            // Track per-channel stats
            for c in 0..3 {
                let ch_diff = (mox[c] as i32 - lcms2_out[c] as i32).abs();
                if ch_diff > 0 {
                    channel_stats[c].max_diff = channel_stats[c].max_diff.max(ch_diff);
                    channel_stats[c].diff_count += 1;
                    channel_stats[c].total_diff += ch_diff as i64;
                }
            }

            max_moxcms_diff = max_moxcms_diff.max(diff);
            diff
        });

        let skcms_diff = skcms_out.map(|sk| {
            let diff: i32 = (0..3)
                .map(|c| (sk[c] as i32 - lcms2_out[c] as i32).abs())
                .max()
                .unwrap();
            max_skcms_diff = max_skcms_diff.max(diff);
            diff
        });

        let qcms_diff = qcms_out.map(|qc| {
            let diff: i32 = (0..3)
                .map(|c| (qc[c] as i32 - lcms2_out[c] as i32).abs())
                .max()
                .unwrap();
            max_qcms_diff = max_qcms_diff.max(diff);
            diff
        });

        // Browser consensus: if skcms and qcms agree within 1, use their value
        let browser_consensus = if let (Some(sk), Some(qc)) = (skcms_out, qcms_out) {
            let agree = (0..3).all(|c| (sk[c] as i32 - qc[c] as i32).abs() <= 1);
            if agree {
                Some(sk)
            } else {
                None
            }
        } else {
            None
        };

        let browser_diff_from_lcms2 = browser_consensus.map(|bc| {
            (0..3)
                .map(|c| (bc[c] as i32 - lcms2_out[c] as i32).abs())
                .max()
                .unwrap()
        });

        let cmp = ColorComparison {
            input: *color,
            lcms2: lcms2_out,
            moxcms: moxcms_out,
            skcms: skcms_out,
            qcms: qcms_out,
            moxcms_diff,
            skcms_diff,
            qcms_diff,
            browser_consensus,
            browser_diff_from_lcms2,
        };

        if cmp.moxcms_matches_browsers() {
            moxcms_better_count += 1;
        }

        comparisons.push(cmp);
    }

    Some(ProfileAnalysis {
        profile_name: filename,
        comparisons,
        max_moxcms_diff,
        max_skcms_diff,
        max_qcms_diff,
        moxcms_better_count,
        channel_diff_stats: channel_stats,
    })
}

#[test]
fn test_v4_profile_diagnostics() {
    eprintln!("\n========================================");
    eprintln!("ICC v4 PROFILE DIAGNOSTICS");
    eprintln!("========================================\n");

    let testdata = testdata_dir();
    let v4_profiles = [
        testdata.join("profiles/skcms/color.org/sRGB_ICC_v4_Appearance.icc"),
        testdata.join("profiles/skcms/color.org/sRGB_v4_ICC_preference.icc"),
        testdata.join("profiles/skcms/misc/sRGB_ICC_v4_beta.icc"),
    ];

    for profile_path in &v4_profiles {
        if !profile_path.exists() {
            eprintln!("SKIP: {} not found", profile_path.display());
            continue;
        }

        eprintln!("\n--- {} ---\n", profile_path.file_name().unwrap().to_string_lossy());

        let analysis = match analyze_profile(profile_path) {
            Some(a) => a,
            None => {
                eprintln!("Failed to analyze profile\n");
                continue;
            }
        };

        eprintln!("Maximum differences vs lcms2:");
        eprintln!("  moxcms: {}", analysis.max_moxcms_diff);
        eprintln!("  skcms:  {}", analysis.max_skcms_diff);
        eprintln!("  qcms:   {}", analysis.max_qcms_diff);

        eprintln!("\nChannel-specific differences (moxcms vs lcms2):");
        for stats in &analysis.channel_diff_stats {
            eprintln!("  {}: max={}, mean={:.2}, count={}/{}",
                stats.channel_name,
                stats.max_diff,
                stats.mean_diff(),
                stats.diff_count,
                TEST_COLORS_RGB8.len()
            );
        }

        eprintln!("\nBrowser consensus analysis:");
        eprintln!("  Times moxcms matches browsers better than lcms2: {}/{}",
            analysis.moxcms_better_count,
            analysis.comparisons.len()
        );

        // Find colors with largest differences
        eprintln!("\nColors with largest moxcms vs lcms2 differences:");
        let mut sorted_comps: Vec<_> = analysis.comparisons.iter()
            .filter_map(|c| c.moxcms_diff.map(|d| (c, d)))
            .collect();
        sorted_comps.sort_by_key(|(_, diff)| -diff);

        for (cmp, diff) in sorted_comps.iter().take(10) {
            if *diff == 0 {
                break;
            }

            let (ch, _ch_diff) = cmp.max_diff_channel().unwrap_or((0, 0));
            let ch_name = ["R", "G", "B"][ch];

            eprintln!("  Input {:?} -> lcms2 {:?}, moxcms {:?} (diff={}, max in {})",
                cmp.input,
                cmp.lcms2,
                cmp.moxcms.unwrap(),
                diff,
                ch_name
            );

            // Show browser outputs if available
            if let Some(consensus) = cmp.browser_consensus {
                eprintln!("    Browser consensus: {:?} (diff from lcms2: {})",
                    consensus,
                    cmp.browser_diff_from_lcms2.unwrap_or(0)
                );
            } else if cmp.skcms.is_some() || cmp.qcms.is_some() {
                eprintln!("    skcms: {:?}, qcms: {:?} (no consensus)",
                    cmp.skcms,
                    cmp.qcms
                );
            }
        }

        eprintln!("\n");
    }

    eprintln!("========================================");
    eprintln!("END OF DIAGNOSTICS");
    eprintln!("========================================\n");
}

/// Check profile structure (matrix vs LUT-based)
#[test]
fn test_v4_profile_structure() {
    eprintln!("\n=== V4 PROFILE STRUCTURE ANALYSIS ===\n");

    let testdata = testdata_dir();
    let v4_profiles = [
        ("sRGB_ICC_v4_Appearance.icc", testdata.join("profiles/skcms/color.org/sRGB_ICC_v4_Appearance.icc")),
        ("sRGB_v4_ICC_preference.icc", testdata.join("profiles/skcms/color.org/sRGB_v4_ICC_preference.icc")),
        ("sRGB_ICC_v4_beta.icc", testdata.join("profiles/skcms/misc/sRGB_ICC_v4_beta.icc")),
    ];

    for (name, profile_path) in &v4_profiles {
        if !profile_path.exists() {
            eprintln!("{}: NOT FOUND", name);
            continue;
        }

        let data = std::fs::read(profile_path).expect("read profile");

        eprintln!("{}:", name);
        eprintln!("  File size: {} bytes", data.len());

        // Check for common ICC tag signatures
        let has_a2b0 = data.windows(4).any(|w| w == b"A2B0");
        let has_b2a0 = data.windows(4).any(|w| w == b"B2A0");
        let has_a2b1 = data.windows(4).any(|w| w == b"A2B1");
        let has_b2a1 = data.windows(4).any(|w| w == b"B2A1");
        let has_a2b2 = data.windows(4).any(|w| w == b"A2B2");
        let has_b2a2 = data.windows(4).any(|w| w == b"B2A2");
        let has_chad = data.windows(4).any(|w| w == b"chad");
        let has_wtpt = data.windows(4).any(|w| w == b"wtpt");
        let has_r_xyz = data.windows(4).any(|w| w == b"rXYZ");
        let has_g_xyz = data.windows(4).any(|w| w == b"gXYZ");
        let has_b_xyz = data.windows(4).any(|w| w == b"bXYZ");
        let has_r_trc = data.windows(4).any(|w| w == b"rTRC");
        let has_g_trc = data.windows(4).any(|w| w == b"gTRC");
        let has_b_trc = data.windows(4).any(|w| w == b"bTRC");

        eprintln!("  Tags present:");
        if has_a2b0 { eprintln!("    A2B0 (Device to PCS - Perceptual)"); }
        if has_b2a0 { eprintln!("    B2A0 (PCS to Device - Perceptual)"); }
        if has_a2b1 { eprintln!("    A2B1 (Device to PCS - Colorimetric)"); }
        if has_b2a1 { eprintln!("    B2A1 (PCS to Device - Colorimetric)"); }
        if has_a2b2 { eprintln!("    A2B2 (Device to PCS - Saturation)"); }
        if has_b2a2 { eprintln!("    B2A2 (PCS to Device - Saturation)"); }
        if has_chad { eprintln!("    chad (Chromatic Adaptation)"); }
        if has_wtpt { eprintln!("    wtpt (White Point)"); }
        if has_r_xyz { eprintln!("    rXYZ (Red Colorant)"); }
        if has_g_xyz { eprintln!("    gXYZ (Green Colorant)"); }
        if has_b_xyz { eprintln!("    bXYZ (Blue Colorant)"); }
        if has_r_trc { eprintln!("    rTRC (Red TRC)"); }
        if has_g_trc { eprintln!("    gTRC (Green TRC)"); }
        if has_b_trc { eprintln!("    bTRC (Blue TRC)"); }

        let profile_type = if has_a2b0 || has_b2a0 || has_a2b1 || has_b2a1 {
            "LUT-based (uses A2B/B2A tables)"
        } else if has_r_xyz && has_g_xyz && has_b_xyz && has_r_trc && has_g_trc && has_b_trc {
            "Matrix-shaper (uses colorant matrix + TRCs)"
        } else {
            "Unknown structure"
        };

        eprintln!("  Profile type: {}", profile_type);

        // Parse with moxcms to check profile class
        if let Ok(_profile) = moxcms::ColorProfile::new_from_slice(&data) {
            eprintln!("  moxcms: Successfully parsed");
        } else {
            eprintln!("  moxcms: Parse failed");
        }

        eprintln!();
    }
}

/// Focused test on specific color ranges
#[test]
fn test_v4_color_range_analysis() {
    eprintln!("\n=== V4 COLOR RANGE ANALYSIS ===\n");

    let testdata = testdata_dir();
    let profile_path = testdata.join("profiles/skcms/color.org/sRGB_ICC_v4_Appearance.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: profile not found");
        return;
    }

    let data = std::fs::read(&profile_path).expect("read profile");

    // Setup CMS
    let srgb_lcms2 = lcms2::Profile::new_srgb();
    let srgb_moxcms = moxcms::ColorProfile::new_srgb();
    let srgb_skcms = skcms_sys::srgb_profile();

    let lcms2_profile = lcms2::Profile::new_icc(&data).expect("lcms2 parse");
    let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).expect("moxcms parse");
    let skcms_profile = skcms_sys::parse_icc_profile(&data).expect("skcms parse");

    let lcms2_transform = lcms2::Transform::new(
        &lcms2_profile,
        lcms2::PixelFormat::RGB_8,
        &srgb_lcms2,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .expect("lcms2 transform");

    let moxcms_transform = moxcms_profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb_moxcms,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("moxcms transform");

    // Test different color ranges
    let ranges = [
        ("Black to dark", 0u8, 32u8),
        ("Dark to mid", 32u8, 128u8),
        ("Mid to bright", 128u8, 224u8),
        ("Bright to white", 224u8, 255u8),
    ];

    for (range_name, start, end) in &ranges {
        eprintln!("\n{} ({}..{}):", range_name, start, end);

        let mut max_diff = 0i32;
        let mut diff_count = 0usize;
        let mut total_diff = 0i64;

        for val in (*start..=*end).step_by(16) {
            // Test grayscale values in this range
            let color = [val, val, val];

            let mut lcms2_out = [0u8; 3];
            lcms2_transform.transform_pixels(&color, &mut lcms2_out);

            let mut moxcms_out = [0u8; 3];
            moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

            let mut skcms_out = [0u8; 3];
            skcms_sys::transform(
                &color,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut skcms_out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                srgb_skcms,
                1,
            );

            let mox_diff: i32 = (0..3)
                .map(|c| (moxcms_out[c] as i32 - lcms2_out[c] as i32).abs())
                .max()
                .unwrap();

            if mox_diff > 0 {
                diff_count += 1;
                total_diff += mox_diff as i64;
                max_diff = max_diff.max(mox_diff);

                eprintln!("  Gray({}) -> lcms2={:?}, moxcms={:?}, skcms={:?}, diff={}",
                    val, lcms2_out, moxcms_out, skcms_out, mox_diff);
            }
        }

        if diff_count > 0 {
            eprintln!("  Range stats: max_diff={}, mean_diff={:.2}",
                max_diff, total_diff as f64 / diff_count as f64);
        } else {
            eprintln!("  No differences found");
        }
    }
}
