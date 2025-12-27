//! CICP Check Test
//!
//! Check if SM245B has CICP metadata that's overriding the TRC.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Check if SM245B has CICP metadata
#[test]
fn check_sm245b_cicp() {
    eprintln!("\n=== SM245B CICP Check ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    // Check CICP
    if let Some(ref cicp) = profile.cicp {
        eprintln!("SM245B has CICP metadata!");
        eprintln!("  CICP: {:?}", cicp);
        eprintln!("\nThis could be the bug - CICP is overriding the TRC LUT!");
    } else {
        eprintln!("SM245B does NOT have CICP metadata");
        eprintln!("The bug is elsewhere...");
    }

    // Also check sRGB for comparison
    let srgb = moxcms::ColorProfile::new_srgb();
    if let Some(ref cicp) = srgb.cicp {
        eprintln!("\nsRGB has CICP metadata: {:?}", cicp);
    } else {
        eprintln!("\nsRGB does NOT have CICP metadata");
    }
}

/// Test with CICP disabled
#[test]
fn test_with_cicp_disabled() {
    eprintln!("\n=== Transform with CICP Disabled ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
    let srgb = moxcms::ColorProfile::new_srgb();

    // Create transform with CICP ENABLED (default)
    let options_cicp = moxcms::TransformOptions {
        allow_use_cicp_transfer: true,
        ..Default::default()
    };

    let transform_cicp = profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            options_cicp,
        )
        .unwrap();

    // Create transform with CICP DISABLED
    let options_no_cicp = moxcms::TransformOptions {
        allow_use_cicp_transfer: false,
        ..Default::default()
    };

    let transform_no_cicp = profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            options_no_cicp,
        )
        .unwrap();

    // Compare outputs
    eprintln!("Comparing CICP enabled vs disabled:");
    eprintln!("Input | CICP on | CICP off | diff");
    eprintln!("------|---------|----------|-----");

    for v in [0u8, 64, 128, 192, 255] {
        let color = [v, v, v];

        let mut out_cicp = [0u8; 3];
        let mut out_no_cicp = [0u8; 3];

        transform_cicp.transform(&color, &mut out_cicp).unwrap();
        transform_no_cicp
            .transform(&color, &mut out_no_cicp)
            .unwrap();

        let diff = out_cicp[0] as i32 - out_no_cicp[0] as i32;
        eprintln!(
            " {:3}  |   {:3}   |   {:3}    | {:+3}",
            v, out_cicp[0], out_no_cicp[0], diff
        );
    }

    // Also compare with skcms reference
    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
        let skcms_srgb = skcms_sys::srgb_profile();

        eprintln!("\nComparing moxcms (CICP off) vs skcms:");
        eprintln!("Input | moxcms | skcms | diff");
        eprintln!("------|--------|-------|-----");

        for v in [0u8, 64, 128, 192, 255] {
            let color = [v, v, v];

            let mut out_mox = [0u8; 3];
            transform_no_cicp.transform(&color, &mut out_mox).unwrap();

            let mut out_skcms = [0u8; 3];
            skcms_sys::transform(
                &color,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut out_skcms,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );

            let diff = out_mox[0] as i32 - out_skcms[0] as i32;
            eprintln!(
                " {:3}  |  {:3}   |  {:3}  | {:+3}",
                v, out_mox[0], out_skcms[0], diff
            );
        }
    }
}
