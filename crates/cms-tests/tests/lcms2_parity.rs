//! Parity tests against lcms2
//!
//! These tests verify that oxcms produces identical output to lcms2
//! for all supported operations.
//!
//! # Known Issue: ARM64 NEON Bug in moxcms
//!
//! On ARM64, moxcms uses fixed-point NEON code paths (`rgb_xyz_q2_13_opt.rs`,
//! `rgb_xyz_q1_30_opt.rs`) that have a copy-paste bug where the blue channel
//! of every second pixel uses the wrong source register:
//!
//! ```ignore
//! // Line 258 in rgb_xyz_q2_13_opt.rs - should use vr1 not vr0!
//! dst0[dst_cn.b_i() + dst_channels] =
//!     self.profile.gamma[vget_lane_u16::<2>(vr0) as usize];  // BUG: uses vr0
//! ```
//!
//! This causes deltaE errors up to 40+ on ARM64. The SSE version (which
//! processes one pixel at a time) does not have this bug.
//!
//! Upstream issue should be filed at: https://github.com/awxkee/moxcms

use cms_tests::accuracy::compare_rgb_buffers;
use cms_tests::patterns::{TestPattern, generate_pattern, sizes};
use cms_tests::reference::{transform_lcms2_srgb, transform_moxcms_srgb};

/// Maximum acceptable deltaE for parity tests.
///
/// On x86/x86_64, differences should be imperceptible (< 1.0 deltaE).
///
/// On ARM64, there's currently an upstream bug in moxcms that causes
/// huge errors (see module docs). These tests will fail on ARM64 until
/// the upstream bug is fixed. We keep the threshold at 1.0 to surface
/// the issue rather than hide it with a larger threshold.
const PARITY_DELTA_E_THRESHOLD: f64 = 1.0;

/// Compare moxcms and lcms2 for sRGB identity transform
#[test]
fn test_srgb_identity_parity() {
    let patterns = [
        TestPattern::GradientH,
        TestPattern::GradientV,
        TestPattern::HueRamp,
        TestPattern::Grayscale,
        TestPattern::SkinTones,
        TestPattern::GamutBoundary,
        TestPattern::Random(42),
        TestPattern::Random(12345),
        TestPattern::Black,
        TestPattern::White,
    ];

    for pattern in patterns {
        let (w, h) = sizes::SMALL;
        let input = generate_pattern(pattern, w, h);

        let moxcms_output = transform_moxcms_srgb(&input).expect("moxcms failed");
        let lcms2_output = transform_lcms2_srgb(&input).expect("lcms2 failed");

        let stats = compare_rgb_buffers(&lcms2_output, &moxcms_output);

        assert!(
            stats.max < PARITY_DELTA_E_THRESHOLD,
            "Pattern {:?}: deltaE mean={:.4}, max={:.4} (threshold={:.1})",
            pattern,
            stats.mean,
            stats.max,
            PARITY_DELTA_E_THRESHOLD
        );
    }
}

/// Test that transforms produce identical output for various image sizes
#[test]
fn test_various_sizes() {
    let test_sizes = [
        sizes::TINY,
        sizes::SMALL,
        sizes::MEDIUM,
        (100, 100),
        (333, 222), // Non-power-of-2
    ];

    for (w, h) in test_sizes {
        let input = generate_pattern(TestPattern::Random(42), w, h);

        let moxcms_output = transform_moxcms_srgb(&input).expect("moxcms failed");
        let lcms2_output = transform_lcms2_srgb(&input).expect("lcms2 failed");

        let stats = compare_rgb_buffers(&lcms2_output, &moxcms_output);

        assert!(
            stats.max < PARITY_DELTA_E_THRESHOLD,
            "Size {}x{}: deltaE mean={:.4}, max={:.4} (threshold={:.1})",
            w,
            h,
            stats.mean,
            stats.max,
            PARITY_DELTA_E_THRESHOLD
        );
    }
}

/// Document any math differences found
/// This test always passes but logs differences for documentation
#[test]
fn document_math_differences() {
    let input = generate_pattern(TestPattern::HueRamp, 256, 1);

    let moxcms_output = transform_moxcms_srgb(&input).expect("moxcms failed");
    let lcms2_output = transform_lcms2_srgb(&input).expect("lcms2 failed");

    // Find pixels where outputs differ
    let mut differences = Vec::new();
    for i in 0..(input.len() / 3) {
        let idx = i * 3;
        let mox = [
            moxcms_output[idx],
            moxcms_output[idx + 1],
            moxcms_output[idx + 2],
        ];
        let lcms = [
            lcms2_output[idx],
            lcms2_output[idx + 1],
            lcms2_output[idx + 2],
        ];

        if mox != lcms {
            differences.push((i, mox, lcms));
        }
    }

    if !differences.is_empty() {
        eprintln!("\n=== MATH DIFFERENCES: moxcms vs lcms2 ===");
        eprintln!(
            "Found {} differing pixels out of {}",
            differences.len(),
            input.len() / 3
        );
        for (i, mox, lcms) in differences.iter().take(10) {
            let input_rgb = [input[i * 3], input[i * 3 + 1], input[i * 3 + 2]];
            eprintln!(
                "  Pixel {}: input={:?} -> moxcms={:?}, lcms2={:?}",
                i, input_rgb, mox, lcms
            );
        }
        if differences.len() > 10 {
            eprintln!("  ... and {} more", differences.len() - 10);
        }
        eprintln!("==========================================\n");
    }
}

/// Diagnostic test for ARM64 - prints actual values for key colors
/// This test always passes but logs detailed diagnostics
#[test]
fn diagnose_arm64_differences() {
    eprintln!("\n=== ARM64 DIAGNOSTIC: sRGB Identity Transform ===");
    eprintln!("Architecture: {}", std::env::consts::ARCH);
    eprintln!("OS: {}", std::env::consts::OS);

    // Test primaries and key values
    let test_colors: &[([u8; 3], &str)] = &[
        ([0, 0, 0], "Black"),
        ([255, 255, 255], "White"),
        ([255, 0, 0], "Red"),
        ([0, 255, 0], "Green"),
        ([0, 0, 255], "Blue"),
        ([128, 128, 128], "Mid Gray"),
        ([1, 1, 1], "Near Black"),
        ([254, 254, 254], "Near White"),
        ([255, 128, 0], "Orange"),
        ([128, 0, 255], "Purple"),
    ];

    let mut input = Vec::new();
    for (rgb, _) in test_colors {
        input.extend_from_slice(rgb);
    }

    let moxcms_output = transform_moxcms_srgb(&input).expect("moxcms failed");
    let lcms2_output = transform_lcms2_srgb(&input).expect("lcms2 failed");

    eprintln!("\nColor       | Input       | moxcms      | lcms2       | Match?");
    eprintln!("------------|-------------|-------------|-------------|-------");

    for (i, (input_rgb, name)) in test_colors.iter().enumerate() {
        let idx = i * 3;
        let mox = [
            moxcms_output[idx],
            moxcms_output[idx + 1],
            moxcms_output[idx + 2],
        ];
        let lcms = [
            lcms2_output[idx],
            lcms2_output[idx + 1],
            lcms2_output[idx + 2],
        ];
        let matches = mox == lcms;

        eprintln!(
            "{:11} | {:3},{:3},{:3} | {:3},{:3},{:3} | {:3},{:3},{:3} | {}",
            name,
            input_rgb[0],
            input_rgb[1],
            input_rgb[2],
            mox[0],
            mox[1],
            mox[2],
            lcms[0],
            lcms[1],
            lcms[2],
            if matches { "✓" } else { "✗" }
        );
    }

    eprintln!("\nIdentity check (should output = input for sRGB->sRGB):");
    for (i, (input_rgb, name)) in test_colors.iter().enumerate() {
        let idx = i * 3;
        let mox = [
            moxcms_output[idx],
            moxcms_output[idx + 1],
            moxcms_output[idx + 2],
        ];
        let lcms = [
            lcms2_output[idx],
            lcms2_output[idx + 1],
            lcms2_output[idx + 2],
        ];

        let mox_identity = mox == *input_rgb;
        let lcms_identity = lcms == *input_rgb;

        if !mox_identity || !lcms_identity {
            eprintln!(
                "  {}: input={:?} moxcms={:?}({}) lcms2={:?}({})",
                name,
                input_rgb,
                mox,
                if mox_identity { "ok" } else { "WRONG" },
                lcms,
                if lcms_identity { "ok" } else { "WRONG" }
            );
        }
    }

    eprintln!("=================================================\n");
}
