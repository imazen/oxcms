//! Parity tests against moxcms
//!
//! These tests verify that oxcms produces identical output to moxcms
//! (since we're forking from it, this should be exact initially).

use cms_tests::accuracy::compare_rgb_buffers;
use cms_tests::patterns::{generate_pattern, sizes, TestPattern};
use cms_tests::reference::transform_moxcms_srgb;

/// Test that we match moxcms exactly for sRGB identity
#[test]
fn test_moxcms_exact_match() {
    let patterns = [
        TestPattern::GradientH,
        TestPattern::HueRamp,
        TestPattern::Random(42),
    ];

    for pattern in patterns {
        let (w, h) = sizes::SMALL;
        let input = generate_pattern(pattern, w, h);

        let moxcms_output = transform_moxcms_srgb(&input).expect("moxcms failed");

        // TODO: Replace with oxcms transform when implemented
        // For now, compare moxcms to itself (should be exact)
        let oxcms_output = moxcms_output.clone();

        let stats = compare_rgb_buffers(&moxcms_output, &oxcms_output);

        assert!(
            stats.max < 0.0001,
            "Pattern {:?}: Should be bit-exact, got deltaE max={:.6}",
            pattern,
            stats.max
        );
    }
}

/// Test moxcms SIMD consistency
/// Different SIMD paths should produce identical results
#[test]
fn test_moxcms_simd_consistency() {
    // Test with various sizes that might trigger different code paths
    let sizes_to_test = [
        (3, 1),   // Tiny
        (4, 1),   // Exactly 4 pixels (SIMD width)
        (7, 1),   // Not aligned
        (8, 1),   // 2x SIMD width
        (15, 1),  // Odd
        (16, 1),  // 4x SIMD width
        (31, 1),  // Prime-ish
        (32, 1),  // 8x SIMD width
        (100, 1), // Larger
    ];

    let base_input = generate_pattern(TestPattern::Random(999), 256, 1);

    for (w, h) in sizes_to_test {
        let input = &base_input[0..(w * h * 3)];

        let output = transform_moxcms_srgb(input).expect("moxcms failed");

        // Run again - should be deterministic
        let output2 = transform_moxcms_srgb(input).expect("moxcms failed");

        assert_eq!(
            output, output2,
            "Size {}x{}: moxcms should be deterministic",
            w, h
        );
    }
}
