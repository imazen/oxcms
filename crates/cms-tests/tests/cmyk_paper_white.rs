//! CMYK Paper White - Cross-CMS Comparison
//!
//! This test compares how different CMS libraries handle CMYK [0,0,0,0] (paper white).
//!
//! ## Findings
//!
//! | CMYK Input | moxcms Output | lcms2 Output | skcms Output |
//! |------------|---------------|--------------|--------------|
//! | [0,0,0,0]  | [255,255,255] | [255,255,255]| [252,254,255]|
//!
//! **moxcms and lcms2 agree** - both produce pure white for CMYK [0,0,0,0].
//! **skcms produces a slightly different result** - [252,254,255].
//!
//! ## Impact on libjxl/jxl-rs Parity
//!
//! Since libjxl uses skcms and jxl-rs uses moxcms, CMYK images will have small
//! differences (~3 units) in near-white areas. This is not a bug in either
//! implementation - it's a fundamental difference in how skcms handles CMYK.
//!
//! skcms may be applying additional processing for CMYK paper simulation that
//! lcms2 and moxcms do not.
//!
//! Profile: cmyk_layers.icc (from JPEG XL cmyk_layers.jxl conformance test)

use std::path::PathBuf;

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("external")
        .join("moxcms")
        .join("assets")
}

/// Transform CMYK to RGB using moxcms
fn transform_moxcms(cmyk_profile_data: &[u8], cmyk: [u8; 4]) -> Option<[u8; 3]> {
    let cmyk_profile = moxcms::ColorProfile::new_from_slice(cmyk_profile_data).ok()?;
    let srgb_profile = moxcms::ColorProfile::new_srgb();

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
    transform.transform_pixels(slice::from_ref(&cmyk), slice::from_mut(&mut rgb));
    Some(rgb)
}

/// Transform CMYK to RGB using skcms
/// Note: skcms auto-inverts CMYK (Photoshop convention), so we pre-invert to use ICC convention
fn transform_skcms(cmyk_profile_data: &[u8], cmyk: [u8; 4]) -> Option<[u8; 3]> {
    let cmyk_profile = skcms_sys::parse_icc_profile(cmyk_profile_data)?;

    if cmyk_profile.data_color_space != skcms_sys::skcms_Signature::CMYK as u32 {
        return None;
    }

    let srgb_profile = skcms_sys::srgb_profile();

    // Pre-invert CMYK values (ICC convention -> Photoshop convention for skcms)
    let inverted_cmyk = [255 - cmyk[0], 255 - cmyk[1], 255 - cmyk[2], 255 - cmyk[3]];

    let mut rgba_out = [0u8; 4];
    let success = skcms_sys::transform(
        &inverted_cmyk,
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

/// Verify moxcms matches lcms2 for CMYK paper white
#[test]
fn test_moxcms_lcms2_paper_white_agreement() {
    let profile_path = assets_dir().join("cmyk_layers.icc");
    let profile_data = match std::fs::read(&profile_path) {
        Ok(data) => data,
        Err(_) => {
            eprintln!("SKIP: cmyk_layers.icc not found at {:?}", profile_path);
            return;
        }
    };

    // CMYK [0,0,0,0] = no ink = paper white
    let paper_white_cmyk: [u8; 4] = [0, 0, 0, 0];

    let moxcms_rgb = transform_moxcms(&profile_data, paper_white_cmyk);
    let lcms2_rgb = transform_lcms2(&profile_data, paper_white_cmyk);
    let skcms_rgb = transform_skcms(&profile_data, paper_white_cmyk);

    eprintln!("\n=== CMYK Paper White Cross-CMS Test ===\n");
    eprintln!("Profile: cmyk_layers.icc (from JPEG XL conformance test)");
    eprintln!("Input:   CMYK [0, 0, 0, 0] (no ink = paper white)\n");

    if let Some(rgb) = moxcms_rgb {
        eprintln!("moxcms:  RGB [{}, {}, {}]", rgb[0], rgb[1], rgb[2]);
    } else {
        eprintln!("moxcms:  FAILED");
    }

    if let Some(rgb) = lcms2_rgb {
        eprintln!("lcms2:   RGB [{}, {}, {}]", rgb[0], rgb[1], rgb[2]);
    } else {
        eprintln!("lcms2:   FAILED");
    }

    if let Some(rgb) = skcms_rgb {
        eprintln!("skcms:   RGB [{}, {}, {}]", rgb[0], rgb[1], rgb[2]);
    } else {
        eprintln!("skcms:   FAILED");
    }

    // moxcms should match lcms2 (the industry standard)
    let moxcms_rgb = moxcms_rgb.expect("moxcms transform should succeed");
    let lcms2_rgb = lcms2_rgb.expect("lcms2 transform should succeed");

    let r_diff = (moxcms_rgb[0] as i16 - lcms2_rgb[0] as i16).abs();
    let g_diff = (moxcms_rgb[1] as i16 - lcms2_rgb[1] as i16).abs();
    let b_diff = (moxcms_rgb[2] as i16 - lcms2_rgb[2] as i16).abs();
    let max_diff = r_diff.max(g_diff).max(b_diff);

    eprintln!("\nmoxcms vs lcms2 difference: R={}, G={}, B={}", r_diff, g_diff, b_diff);

    assert!(
        max_diff <= 1,
        "moxcms should match lcms2 for CMYK paper white. \
         Got moxcms [{},{},{}] vs lcms2 [{},{},{}] (diff={}).",
        moxcms_rgb[0], moxcms_rgb[1], moxcms_rgb[2],
        lcms2_rgb[0], lcms2_rgb[1], lcms2_rgb[2],
        max_diff
    );

    // Note the skcms difference for documentation
    if let Some(skcms_rgb) = skcms_rgb {
        let skcms_diff = (0..3)
            .map(|i| (lcms2_rgb[i] as i16 - skcms_rgb[i] as i16).abs())
            .max()
            .unwrap_or(0);

        if skcms_diff > 0 {
            eprintln!("\nNOTE: skcms differs from lcms2/moxcms by {} units.", skcms_diff);
            eprintln!("This explains the jxl-rs vs libjxl parity gap for CMYK images.");
        }
    }

    eprintln!("\nPASS: moxcms matches lcms2 for paper white");
}

/// Document the difference between lcms2 and skcms paper white handling
#[test]
fn test_lcms2_skcms_paper_white_difference() {
    let profile_path = assets_dir().join("cmyk_layers.icc");
    let profile_data = match std::fs::read(&profile_path) {
        Ok(data) => data,
        Err(_) => {
            eprintln!("SKIP: cmyk_layers.icc not found");
            return;
        }
    };

    let paper_white_cmyk: [u8; 4] = [0, 0, 0, 0];

    let lcms2_rgb = transform_lcms2(&profile_data, paper_white_cmyk);
    let skcms_rgb = transform_skcms(&profile_data, paper_white_cmyk);

    eprintln!("\n=== lcms2 vs skcms Paper White Comparison ===\n");

    if let (Some(l), Some(s)) = (lcms2_rgb, skcms_rgb) {
        eprintln!("lcms2: RGB [{}, {}, {}]", l[0], l[1], l[2]);
        eprintln!("skcms: RGB [{}, {}, {}]", s[0], s[1], s[2]);

        let max_diff = (0..3)
            .map(|i| (l[i] as i16 - s[i] as i16).abs())
            .max()
            .unwrap_or(0);

        eprintln!("Max difference: {}", max_diff);

        // Document the known difference - skcms returns slightly off-white
        // for CMYK paper white, while lcms2 returns pure white
        if max_diff > 0 {
            eprintln!("\nNOTE: skcms produces a slightly different paper white than lcms2.");
            eprintln!("This is a known behavior difference, not a bug in either.");
            eprintln!("Impact: libjxl (skcms) vs jxl-rs (moxcms) will differ by ~{} units.", max_diff);
        }

        // This is informational - we just document the difference
        assert!(
            max_diff <= 5,
            "Unexpectedly large difference ({}) between lcms2 and skcms",
            max_diff
        );
    } else {
        panic!("One or both reference implementations failed");
    }
}

/// Test multiple near-white CMYK values
#[test]
fn test_nearwhite_cmyk_values() {
    let profile_path = assets_dir().join("cmyk_layers.icc");
    let profile_data = match std::fs::read(&profile_path) {
        Ok(data) => data,
        Err(_) => {
            eprintln!("SKIP: cmyk_layers.icc not found");
            return;
        }
    };

    // Test values: [C, M, Y, K] in ICC convention (0 = no ink)
    // Include both near-white AND saturated values to find where big errors come from
    let test_cases: &[([u8; 4], &str)] = &[
        ([0, 0, 0, 0], "Paper white"),
        ([1, 0, 0, 0], "1% Cyan"),
        ([5, 2, 0, 0], "5% Cyan, 2% Magenta"),
        // jxl-rs worst error pixel (150,211) - Background layer:
        // ICC C=0.7569 M=0.0 Y=0.3529 K=0.0
        ([193, 0, 90, 0], "Background@(150,211)"),
        // Layer 1 at local (7,45) = absolute (150,211):
        // JXL: [1.0, 0.0, 0.2784, 1.0] → ICC: [0.0, 1.0, 0.7216, 0.0]
        // = [0, 255, 184, 0] in u8
        ([0, 255, 184, 0], "Layer1@(150,211)"),
        ([128, 64, 32, 0], "50% C, 25% M, 12% Y"),
        ([255, 128, 64, 0], "100% C, 50% M, 25% Y"),
        ([64, 255, 128, 0], "25% C, 100% M, 50% Y"),
        ([128, 64, 255, 0], "50% C, 25% M, 100% Y"),
        ([200, 100, 50, 25], "High C with K"),
        ([50, 200, 100, 50], "High M with K"),
        ([100, 50, 200, 75], "High Y with K"),
    ];

    eprintln!("\n=== Near-White CMYK Comparison ===\n");
    eprintln!("{:<30} {:>12} {:>12} {:>12} {:>8}", "Test Case", "moxcms", "lcms2", "skcms", "Δmax");
    eprintln!("{}", "-".repeat(78));

    let mut max_moxcms_error = 0i16;

    for (cmyk, name) in test_cases {
        let moxcms_rgb = transform_moxcms(&profile_data, *cmyk);
        let lcms2_rgb = transform_lcms2(&profile_data, *cmyk);
        let skcms_rgb = transform_skcms(&profile_data, *cmyk);

        let m_str = moxcms_rgb
            .map(|r| format!("[{:3},{:3},{:3}]", r[0], r[1], r[2]))
            .unwrap_or_else(|| "FAIL".to_string());
        let l_str = lcms2_rgb
            .map(|r| format!("[{:3},{:3},{:3}]", r[0], r[1], r[2]))
            .unwrap_or_else(|| "FAIL".to_string());
        let s_str = skcms_rgb
            .map(|r| format!("[{:3},{:3},{:3}]", r[0], r[1], r[2]))
            .unwrap_or_else(|| "FAIL".to_string());

        // Calculate max diff between moxcms and lcms2
        let diff = if let (Some(m), Some(l)) = (moxcms_rgb, lcms2_rgb) {
            let d = (0..3)
                .map(|i| (m[i] as i16 - l[i] as i16).abs())
                .max()
                .unwrap_or(0);
            max_moxcms_error = max_moxcms_error.max(d);
            format!("{}", d)
        } else {
            "-".to_string()
        };

        eprintln!("{:<30} {:>12} {:>12} {:>12} {:>8}", name, m_str, l_str, s_str, diff);
    }

    eprintln!("\nMax moxcms error vs lcms2: {}", max_moxcms_error);

    // Document findings - moxcms should be close to lcms2
    // For saturated colors, small differences are expected due to interpolation
    if max_moxcms_error > 5 {
        eprintln!("WARNING: Large difference ({}) between moxcms and lcms2!", max_moxcms_error);
        eprintln!("This may indicate an interpolation or CLUT lookup issue.");
    }
}
