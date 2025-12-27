//! Per-Channel TRC Investigation
//!
//! Check if SM245B has different TRCs per channel.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Check if SM245B has different TRCs per channel
#[test]
fn check_per_channel_trc() {
    eprintln!("\n=== Per-Channel TRC Check ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    // Check each channel's TRC
    let trcs = [
        ("Red", &profile.red_trc),
        ("Green", &profile.green_trc),
        ("Blue", &profile.blue_trc),
    ];

    for (name, trc) in &trcs {
        if let Some(moxcms::ToneReprCurve::Lut(lut)) = trc {
            // Sample a few values
            let mid = lut.len() / 2;
            eprintln!("{} TRC: {} entries, mid={}", name, lut.len(), lut[mid]);
        } else {
            eprintln!("{} TRC: {:?}", name, trc);
        }
    }

    // Check if all TRCs are the same
    if let (
        Some(moxcms::ToneReprCurve::Lut(r)),
        Some(moxcms::ToneReprCurve::Lut(g)),
        Some(moxcms::ToneReprCurve::Lut(b)),
    ) = (&profile.red_trc, &profile.green_trc, &profile.blue_trc)
    {
        let r_eq_g = r == g;
        let g_eq_b = g == b;
        eprintln!("\nR == G: {}", r_eq_g);
        eprintln!("G == B: {}", g_eq_b);

        if !r_eq_g || !g_eq_b {
            eprintln!("\n==> SM245B has DIFFERENT TRCs per channel! <==");

            // Show the differences at midpoint
            eprintln!("\nMidpoint values:");
            let mid = r.len() / 2;
            eprintln!("  R[{}] = {} ({:.5})", mid, r[mid], r[mid] as f32 / 65535.0);
            eprintln!("  G[{}] = {} ({:.5})", mid, g[mid], g[mid] as f32 / 65535.0);
            eprintln!("  B[{}] = {} ({:.5})", mid, b[mid], b[mid] as f32 / 65535.0);
        }
    }
}

/// Test individual channel transforms
#[test]
fn test_individual_channels() {
    eprintln!("\n=== Individual Channel Transforms ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Use skcms to test pure red, green, blue inputs
    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
        let skcms_srgb = skcms_sys::srgb_profile();

        eprintln!("Pure channel transforms at 128:");

        // Pure red
        let mut red_out = [0u8; 3];
        skcms_sys::transform(
            &[128u8, 0, 0],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut red_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [128, 0, 0] -> {:?}", red_out);

        // Pure green
        let mut green_out = [0u8; 3];
        skcms_sys::transform(
            &[0u8, 128, 0],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut green_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [0, 128, 0] -> {:?}", green_out);

        // Pure blue
        let mut blue_out = [0u8; 3];
        skcms_sys::transform(
            &[0u8, 0, 128],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut blue_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [0, 0, 128] -> {:?}", blue_out);

        // Gray
        let mut gray_out = [0u8; 3];
        skcms_sys::transform(
            &[128u8, 128, 128],
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            &skcms_profile,
            &mut gray_out,
            skcms_sys::skcms_PixelFormat::RGB_888,
            skcms_sys::skcms_AlphaFormat::Opaque,
            skcms_srgb,
            1,
        );
        eprintln!("  [128, 128, 128] -> {:?}", gray_out);

        eprintln!("\nNote: If gray output is not neutral, TRCs differ per channel");
    }

    // Now test moxcms
    let mox_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let mox_transform = mox_profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &mox_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .unwrap();

    eprintln!("\nmoxcms transforms:");
    for input in [[128u8, 0, 0], [0, 128, 0], [0, 0, 128], [128, 128, 128]] {
        let mut out = [0u8; 3];
        mox_transform.transform(&input, &mut out).unwrap();
        eprintln!("  {:?} -> {:?}", input, out);
    }
}

/// Detailed TRC comparison at multiple points
#[test]
fn detailed_trc_comparison() {
    eprintln!("\n=== Detailed TRC Per-Channel Comparison ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    if let (
        Some(moxcms::ToneReprCurve::Lut(r)),
        Some(moxcms::ToneReprCurve::Lut(g)),
        Some(moxcms::ToneReprCurve::Lut(b)),
    ) = (&profile.red_trc, &profile.green_trc, &profile.blue_trc)
    {
        eprintln!("TRC values at key indices:");
        eprintln!("Index |   Red   |  Green  |  Blue   | R-G diff | G-B diff");
        eprintln!("------|---------|---------|---------|----------|----------");

        for idx in (0..=255).step_by(32) {
            let r_val = r[idx];
            let g_val = g[idx];
            let b_val = b[idx];
            let rg_diff = r_val as i32 - g_val as i32;
            let gb_diff = g_val as i32 - b_val as i32;

            eprintln!(
                " {:3}  | {:5}   | {:5}   | {:5}   |   {:+5}  |   {:+5}",
                idx, r_val, g_val, b_val, rg_diff, gb_diff
            );
        }
    }
}
