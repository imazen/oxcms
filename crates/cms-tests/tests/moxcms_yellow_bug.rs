//! Reproducible test case for moxcms CMYK pure yellow axis bug
//!
//! This test demonstrates that moxcms produces different results than lcms2 and skcms
//! for pure yellow CMYK values [0,0,Y,0] where Y > 0.
//!
//! ## Bug Summary
//!
//! When transforming CMYK to sRGB using a standard CMYK profile (USWebCoatedSWOP.icc),
//! moxcms produces green channel values that are 3-8 lower than both lcms2 and skcms.
//!
//! ## Expected vs Actual
//!
//! For CMYK [0,0,192,0] (pure yellow, 75%):
//! - lcms2:  RGB(255, 244, 94)
//! - skcms:  RGB(255, 243, 95)
//! - moxcms: RGB(255, 237, 94)  ← Green channel is 7 lower
//!
//! ## Key Observations
//!
//! 1. lcms2 and skcms agree (Δ ≤ 1)
//! 2. moxcms diverges by 3-8 depending on Y value
//! 3. Switching interpolation method (trilinear vs tetrahedral) makes NO difference
//! 4. Bug scales linearly with Y value
//!
//! ## Profile
//!
//! USWebCoatedSWOP.icc (U.S. Web Coated SWOP v2) - standard Adobe CMYK profile
//! Available from Adobe or color.org

use std::path::PathBuf;

fn icc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/icc")
}

/// Minimal reproduction: moxcms green channel is too low for pure yellow CMYK
#[test]
fn test_moxcms_pure_yellow_bug() {
    let profile_path = icc_dir().join("USWebCoatedSWOP.icc");
    let profile_data = std::fs::read(&profile_path).expect("USWebCoatedSWOP.icc required");

    // Test case: CMYK [0, 0, 192, 0] = 75% yellow, no other inks
    let cmyk_input: [u8; 4] = [0, 0, 192, 0];

    // Transform with moxcms
    let moxcms_rgb = {
        use moxcms::{ColorProfile, Layout, TransformOptions};

        let cmyk_profile = ColorProfile::new_from_slice(&profile_data).unwrap();
        let srgb = ColorProfile::new_srgb();

        let transform = cmyk_profile
            .create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgb, TransformOptions::default())
            .unwrap();

        let mut rgb = [0u8; 3];
        transform.transform(&cmyk_input, &mut rgb).unwrap();
        rgb
    };

    // Transform with lcms2 (reference)
    let lcms2_rgb = {
        use lcms2::{Intent, PixelFormat, Profile, Transform};
        use std::slice;

        let cmyk_profile = Profile::new_icc(&profile_data).unwrap();
        let srgb = Profile::new_srgb();

        let transform = Transform::<[u8; 4], [u8; 3]>::new(
            &cmyk_profile,
            PixelFormat::CMYK_8,
            &srgb,
            PixelFormat::RGB_8,
            Intent::Perceptual,
        ).unwrap();

        let mut rgb = [0u8; 3];
        transform.transform_pixels(slice::from_ref(&cmyk_input), slice::from_mut(&mut rgb));
        rgb
    };

    println!("\n=== moxcms Pure Yellow Bug Reproduction ===\n");
    println!("Input CMYK: {:?}", cmyk_input);
    println!("lcms2 RGB:  {:?}", lcms2_rgb);
    println!("moxcms RGB: {:?}", moxcms_rgb);

    let green_diff = (lcms2_rgb[1] as i16 - moxcms_rgb[1] as i16).abs();
    println!("\nGreen channel difference: {} (expected: 0-1)", green_diff);

    // The bug: moxcms green channel is ~7 lower than lcms2
    // Expected: lcms2_rgb[1] ≈ 244, moxcms_rgb[1] ≈ 237
    assert!(
        green_diff <= 1,
        "BUG: moxcms green channel differs by {} from lcms2 (expected ≤1)\n\
         lcms2:  {:?}\n\
         moxcms: {:?}",
        green_diff,
        lcms2_rgb,
        moxcms_rgb
    );
}

/// Full test showing the bug scales with Y value
#[test]
fn test_moxcms_yellow_bug_scaling() {
    let profile_path = icc_dir().join("USWebCoatedSWOP.icc");
    let profile_data = std::fs::read(&profile_path).expect("USWebCoatedSWOP.icc required");

    println!("\n=== moxcms Yellow Bug: Scaling with Y value ===\n");
    println!("{:<12} {:>12} {:>12} {:>8}", "CMYK[Y]", "lcms2 G", "moxcms G", "Δ");
    println!("{}", "-".repeat(48));

    let mut max_diff = 0i16;

    for y in (0..=255).step_by(32) {
        let cmyk: [u8; 4] = [0, 0, y as u8, 0];

        // moxcms
        let moxcms_g = {
            use moxcms::{ColorProfile, Layout, TransformOptions};
            let cmyk_profile = ColorProfile::new_from_slice(&profile_data).unwrap();
            let srgb = ColorProfile::new_srgb();
            let transform = cmyk_profile
                .create_transform_8bit(Layout::Rgba, &srgb, Layout::Rgb, TransformOptions::default())
                .unwrap();
            let mut rgb = [0u8; 3];
            transform.transform(&cmyk, &mut rgb).unwrap();
            rgb[1]
        };

        // lcms2
        let lcms2_g = {
            use lcms2::{Intent, PixelFormat, Profile, Transform};
            use std::slice;
            let cmyk_profile = Profile::new_icc(&profile_data).unwrap();
            let srgb = Profile::new_srgb();
            let transform = Transform::<[u8; 4], [u8; 3]>::new(
                &cmyk_profile, PixelFormat::CMYK_8,
                &srgb, PixelFormat::RGB_8,
                Intent::Perceptual,
            ).unwrap();
            let mut rgb = [0u8; 3];
            transform.transform_pixels(slice::from_ref(&cmyk), slice::from_mut(&mut rgb));
            rgb[1]
        };

        let diff = (lcms2_g as i16 - moxcms_g as i16).abs();
        if diff > max_diff {
            max_diff = diff;
        }

        println!("[0,0,{:3},0] {:>12} {:>12} {:>8}", y, lcms2_g, moxcms_g, diff);
    }

    println!("\nMax green channel difference: {}", max_diff);
    println!("Note: Difference scales approximately linearly with Y value");

    // This assertion will fail, demonstrating the bug
    assert!(
        max_diff <= 1,
        "BUG: max green channel diff is {} (expected ≤1)",
        max_diff
    );
}
