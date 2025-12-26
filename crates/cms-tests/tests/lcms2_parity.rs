//! Parity tests against lcms2
//!
//! These tests verify that oxcms produces identical output to lcms2
//! for all supported operations.

use cms_tests::accuracy::compare_rgb_buffers;
use cms_tests::patterns::{generate_pattern, sizes, TestPattern};
use cms_tests::reference::{transform_lcms2_srgb, transform_moxcms_srgb};

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
            stats.is_excellent(),
            "Pattern {:?}: deltaE mean={:.4}, max={:.4} (should be < 1.0)",
            pattern,
            stats.mean,
            stats.max
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
            stats.is_excellent(),
            "Size {}x{}: deltaE mean={:.4}, max={:.4}",
            w,
            h,
            stats.mean,
            stats.max
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
        let mox = [moxcms_output[idx], moxcms_output[idx + 1], moxcms_output[idx + 2]];
        let lcms = [lcms2_output[idx], lcms2_output[idx + 1], lcms2_output[idx + 2]];

        if mox != lcms {
            differences.push((i, mox, lcms));
        }
    }

    if !differences.is_empty() {
        eprintln!("\n=== MATH DIFFERENCES: moxcms vs lcms2 ===");
        eprintln!("Found {} differing pixels out of {}", differences.len(), input.len() / 3);
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
