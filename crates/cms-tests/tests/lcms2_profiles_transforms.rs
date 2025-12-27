//! Profile and Transform tests ported from lcms2 testbed
//!
//! These tests cover profile creation, transforms, and known values.
//!
//! Original source: https://github.com/mm2/Little-CMS/blob/master/testbed/testcms2.c

use lcms2::{CIExyY, CIExyYTRIPLE, Intent, PixelFormat, Profile, ToneCurve};
use std::slice;

// ============================================================================
// Profile Creation Tests
// ============================================================================

/// Test sRGB profile creation
#[test]
fn test_create_srgb_profile() {
    let srgb = Profile::new_srgb();

    // sRGB should be an RGB color space
    assert_eq!(
        srgb.color_space(),
        lcms2::ColorSpaceSignature::RgbData,
        "sRGB should have RGB color space"
    );

    // sRGB should be a Display device class
    assert_eq!(
        srgb.device_class(),
        lcms2::ProfileClassSignature::DisplayClass,
        "sRGB should be Display class"
    );

    // sRGB should have version 2.x or 4.x
    let version = srgb.version();
    assert!(
        (2.0..5.0).contains(&version),
        "sRGB version should be 2.x-4.x, got {}",
        version
    );
}

/// Test XYZ profile creation
#[test]
fn test_create_xyz_profile() {
    let xyz = Profile::new_xyz();

    assert_eq!(
        xyz.color_space(),
        lcms2::ColorSpaceSignature::XYZData,
        "XYZ profile should have XYZ color space"
    );
}

/// Test gray profile creation
#[test]
fn test_create_gray_profile() {
    let d50 = CIExyY {
        x: 0.3457,
        y: 0.3585,
        Y: 1.0,
    };
    let gamma = ToneCurve::new(2.2);

    let gray = Profile::new_gray(&d50, &gamma).expect("Gray profile creation failed");

    assert_eq!(
        gray.color_space(),
        lcms2::ColorSpaceSignature::GrayData,
        "Gray profile should have Gray color space"
    );
}

/// Test custom RGB profile creation
#[test]
fn test_create_rgb_profile() {
    // D65 white point
    let white = CIExyY {
        x: 0.3127,
        y: 0.3290,
        Y: 1.0,
    };

    // sRGB-like primaries
    let primaries = CIExyYTRIPLE {
        Red: CIExyY {
            x: 0.64,
            y: 0.33,
            Y: 1.0,
        },
        Green: CIExyY {
            x: 0.30,
            y: 0.60,
            Y: 1.0,
        },
        Blue: CIExyY {
            x: 0.15,
            y: 0.06,
            Y: 1.0,
        },
    };

    let gamma = ToneCurve::new(2.2);
    let curves = [&gamma, &gamma, &gamma];

    let rgb = Profile::new_rgb(&white, &primaries, &curves).expect("RGB profile creation failed");

    assert_eq!(
        rgb.color_space(),
        lcms2::ColorSpaceSignature::RgbData,
        "RGB profile should have RGB color space"
    );
}

// ============================================================================
// sRGB Primaries Test
// ============================================================================

/// Test that sRGB profile transforms primaries correctly to D50 PCS
/// Port of CheckRGBPrimaries from testcms2.c
/// Note: sRGB primaries are D65-relative, but ICC XYZ PCS is D50.
/// The transform chromatic-adapts the primaries to D50.
#[test]
fn test_srgb_primaries() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    // Create transform from sRGB to XYZ (float)
    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::AbsoluteColorimetric,
    )
    .expect("Transform creation failed");

    // Transform RGB primaries
    let red_rgb = [1.0f32, 0.0, 0.0];
    let green_rgb = [0.0f32, 1.0, 0.0];
    let blue_rgb = [0.0f32, 0.0, 1.0];

    let mut red_xyz = [0.0f32; 3];
    let mut green_xyz = [0.0f32; 3];
    let mut blue_xyz = [0.0f32; 3];

    transform.transform_pixels(slice::from_ref(&red_rgb), slice::from_mut(&mut red_xyz));
    transform.transform_pixels(slice::from_ref(&green_rgb), slice::from_mut(&mut green_xyz));
    transform.transform_pixels(slice::from_ref(&blue_rgb), slice::from_mut(&mut blue_xyz));

    // Convert XYZ to xyY (x = X/(X+Y+Z), y = Y/(X+Y+Z))
    fn xyz_to_xy(xyz: [f32; 3]) -> (f32, f32) {
        let sum = xyz[0] + xyz[1] + xyz[2];
        if sum < 1e-10 {
            return (0.0, 0.0);
        }
        (xyz[0] / sum, xyz[1] / sum)
    }

    let (red_x, red_y) = xyz_to_xy(red_xyz);
    let (green_x, green_y) = xyz_to_xy(green_xyz);
    let (blue_x, blue_y) = xyz_to_xy(blue_xyz);

    // After chromatic adaptation from D65 to D50, the primaries shift.
    // D50-adapted sRGB primaries (approximate):
    // Red: x≈0.648, y≈0.331
    // Green: x≈0.321, y≈0.598
    // Blue: x≈0.156, y≈0.066
    let tolerance = 0.02; // 2% tolerance due to chromatic adaptation

    assert!(
        (red_x - 0.648).abs() < tolerance,
        "Red x should be ~0.648 (D50), got {}",
        red_x
    );
    assert!(
        (red_y - 0.331).abs() < tolerance,
        "Red y should be ~0.331 (D50), got {}",
        red_y
    );

    assert!(
        (green_x - 0.321).abs() < tolerance,
        "Green x should be ~0.321 (D50), got {}",
        green_x
    );
    assert!(
        (green_y - 0.598).abs() < tolerance,
        "Green y should be ~0.598 (D50), got {}",
        green_y
    );

    assert!(
        (blue_x - 0.156).abs() < tolerance,
        "Blue x should be ~0.156 (D50), got {}",
        blue_x
    );
    assert!(
        (blue_y - 0.066).abs() < tolerance,
        "Blue y should be ~0.066 (D50), got {}",
        blue_y
    );
}

// ============================================================================
// Known sRGB Values Tests
// ============================================================================

/// Test known RGB to XYZ values
/// Port of Chack_sRGB_Float from testcms2.c
#[test]
fn test_srgb_to_xyz_known_values() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // Test known values from lcms2 testbed
    // Format: (R/255, G/255, B/255) -> (X, Y, Z) with tolerance
    let test_cases = [
        // Near-black: RGB(1,1,1) -> XYZ(0.0003, 0.0003, 0.0003)
        (
            [1.0f32 / 255.0, 1.0 / 255.0, 1.0 / 255.0],
            [0.0002927, 0.0003035, 0.000250],
            0.0002,
        ),
        // Mid-gray: RGB(127,127,127) -> XYZ(0.205, 0.212, 0.175)
        (
            [127.0f32 / 255.0, 127.0 / 255.0, 127.0 / 255.0],
            [0.2046329, 0.212230, 0.175069],
            0.01,
        ),
        // Dark color: RGB(12,13,15)
        (
            [12.0f32 / 255.0, 13.0 / 255.0, 15.0 / 255.0],
            [0.0038364, 0.0039928, 0.003853],
            0.001,
        ),
        // Red: RGB(128,0,0)
        (
            [128.0f32 / 255.0, 0.0, 0.0],
            [0.0941240, 0.0480256, 0.003005],
            0.01,
        ),
        // Purple: RGB(190,25,210)
        (
            [190.0f32 / 255.0, 25.0 / 255.0, 210.0 / 255.0],
            [0.3204592, 0.1605926, 0.468213],
            0.01,
        ),
    ];

    for (rgb, expected_xyz, tolerance) in test_cases {
        let mut result = [0.0f32; 3];
        transform.transform_pixels(slice::from_ref(&rgb), slice::from_mut(&mut result));

        for i in 0..3 {
            let err = (result[i] - expected_xyz[i] as f32).abs();
            assert!(
                err < tolerance as f32,
                "RGB({:.3},{:.3},{:.3}) XYZ[{}]: expected {}, got {}, error {}",
                rgb[0] * 255.0,
                rgb[1] * 255.0,
                rgb[2] * 255.0,
                i,
                expected_xyz[i],
                result[i],
                err
            );
        }
    }
}

// ============================================================================
// Transform Tests
// ============================================================================

/// Test sRGB identity transform
#[test]
fn test_srgb_identity_transform() {
    let srgb = Profile::new_srgb();

    // sRGB -> sRGB should be identity
    let transform = lcms2::Transform::<[u8; 3], [u8; 3]>::new(
        &srgb,
        PixelFormat::RGB_8,
        &srgb,
        PixelFormat::RGB_8,
        Intent::Perceptual,
    )
    .expect("Transform creation failed");

    // Test all gray values
    for v in 0u8..=255 {
        let input = [v, v, v];
        let mut output = [0u8; 3];
        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        // Allow 2 levels of error (rounding)
        let max_err = (output[0] as i32 - v as i32)
            .abs()
            .max((output[1] as i32 - v as i32).abs())
            .max((output[2] as i32 - v as i32).abs());

        assert!(
            max_err <= 2,
            "Gray {} identity transform error: got {:?}, max_err={}",
            v,
            output,
            max_err
        );
    }
}

/// Test 16-bit transform precision (sRGB to sRGB identity)
#[test]
fn test_transform_16bit() {
    let srgb = Profile::new_srgb();

    // sRGB -> sRGB identity transform in 16-bit
    let transform = lcms2::Transform::<[u16; 3], [u16; 3]>::new(
        &srgb,
        PixelFormat::RGB_16,
        &srgb,
        PixelFormat::RGB_16,
        Intent::RelativeColorimetric,
    )
    .expect("Transform failed");

    // Test at several values
    let test_values: [u16; 5] = [0, 16384, 32768, 49152, 65535];

    for &v in &test_values {
        let input = [v, v, v];
        let mut output = [0u16; 3];

        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        // Identity should be very precise - allow 0x100 error
        let max_err = (output[0] as i32 - v as i32)
            .abs()
            .max((output[1] as i32 - v as i32).abs())
            .max((output[2] as i32 - v as i32).abs());

        assert!(
            max_err <= 0x100,
            "16-bit identity at {}: got {:?}, max_err={}",
            v,
            output,
            max_err
        );
    }
}

/// Test float transform precision
#[test]
fn test_transform_float() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    // Forward: sRGB -> XYZ
    let forward = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Forward transform failed");

    // Inverse: XYZ -> sRGB
    let inverse = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &xyz,
        PixelFormat::XYZ_FLT,
        &srgb,
        PixelFormat::RGB_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Inverse transform failed");

    // Test roundtrip at several values
    for i in 0..=10 {
        let v = i as f32 / 10.0;
        let input = [v, v, v];
        let mut xyz_out = [0.0f32; 3];
        let mut rgb_out = [0.0f32; 3];

        forward.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut xyz_out));
        inverse.transform_pixels(slice::from_ref(&xyz_out), slice::from_mut(&mut rgb_out));

        // Float should be very precise
        let max_err = (rgb_out[0] - v)
            .abs()
            .max((rgb_out[1] - v).abs())
            .max((rgb_out[2] - v).abs());

        assert!(
            max_err < 0.001,
            "Float roundtrip at {}: got {:?}, max_err={}",
            v,
            rgb_out,
            max_err
        );
    }
}

// ============================================================================
// Profile ICC Serialization Tests
// ============================================================================

/// Test that sRGB can be serialized to ICC and reloaded
#[test]
fn test_profile_icc_roundtrip() {
    let srgb = Profile::new_srgb();

    // Get ICC data
    let icc_data = srgb.icc().expect("ICC serialization failed");

    // Should have reasonable size (sRGB is typically 3-4 KB)
    assert!(
        icc_data.len() > 100,
        "ICC data too small: {} bytes",
        icc_data.len()
    );
    assert!(
        icc_data.len() < 100_000,
        "ICC data too large: {} bytes",
        icc_data.len()
    );

    // Should be valid ICC (starts with profile size and magic)
    assert_eq!(
        &icc_data[36..40],
        b"acsp",
        "ICC should have 'acsp' magic at offset 36"
    );

    // Reload the profile
    let reloaded = Profile::new_icc(&icc_data).expect("ICC reload failed");

    // Should have same properties
    assert_eq!(
        srgb.color_space(),
        reloaded.color_space(),
        "Color space should match after reload"
    );
    assert_eq!(
        srgb.device_class(),
        reloaded.device_class(),
        "Device class should match after reload"
    );
}

// ============================================================================
// Gray Profile Transform Tests
// ============================================================================

/// Test gray profile identity transform
#[test]
fn test_gray_transform() {
    let d50 = CIExyY {
        x: 0.3457,
        y: 0.3585,
        Y: 1.0,
    };

    let gamma_22 = ToneCurve::new(2.2);
    let gray = Profile::new_gray(&d50, &gamma_22).expect("Gray profile failed");

    // Gray -> Gray (same profile) should be identity
    let transform = lcms2::Transform::<[u8; 1], [u8; 1]>::new(
        &gray,
        PixelFormat::GRAY_8,
        &gray,
        PixelFormat::GRAY_8,
        Intent::Perceptual,
    )
    .expect("Transform creation failed");

    // Test that identity transform preserves values
    for i in 0..=255 {
        let input = [i as u8];
        let mut output = [0u8];
        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        // Allow 2 levels error due to rounding
        let err = (output[0] as i32 - i).abs();
        assert!(
            err <= 2,
            "Gray identity at {}: got {}, error {}",
            i,
            output[0],
            err
        );
    }
}

// ============================================================================
// Rendering Intent Tests
// ============================================================================

/// Test different rendering intents produce valid results
#[test]
fn test_rendering_intents() {
    let srgb = Profile::new_srgb();

    // Create transforms with different intents
    let intents = [
        Intent::Perceptual,
        Intent::RelativeColorimetric,
        Intent::Saturation,
        Intent::AbsoluteColorimetric,
    ];

    for intent in intents {
        let transform = lcms2::Transform::<[u8; 3], [u8; 3]>::new(
            &srgb,
            PixelFormat::RGB_8,
            &srgb,
            PixelFormat::RGB_8,
            intent,
        )
        .expect("Transform creation failed");

        // Test a saturated color
        let input = [255u8, 0, 128];
        let mut output = [0u8; 3];
        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        // All should produce valid RGB
        assert!(
            output[0] > 0 || output[1] > 0 || output[2] > 0,
            "Output should not be black for intent {:?}",
            intent
        );
    }
}

/// Test profile version handling
#[test]
fn test_profile_versions() {
    // sRGB should be v2 or v4
    let srgb = Profile::new_srgb();
    let version = srgb.version();
    assert!(version >= 2.0, "sRGB version should be >= 2.0");

    // XYZ profile version
    let xyz = Profile::new_xyz();
    let xyz_version = xyz.version();
    assert!(xyz_version >= 2.0, "XYZ version should be >= 2.0");
}

// ============================================================================
// sRGB Transform Accuracy Tests
// ============================================================================

/// Test sRGB -> XYZ -> sRGB roundtrip (float) at various values
#[test]
fn test_srgb_xyz_roundtrip_float() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    let forward = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Forward transform failed");

    let inverse = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &xyz,
        PixelFormat::XYZ_FLT,
        &srgb,
        PixelFormat::RGB_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Inverse transform failed");

    let mut max_error = 0.0f32;

    // Test gray values
    for i in 0..=255 {
        let v = i as f32 / 255.0;
        let input = [v, v, v];
        let mut xyz_out = [0.0f32; 3];
        let mut rgb_out = [0.0f32; 3];

        forward.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut xyz_out));
        inverse.transform_pixels(slice::from_ref(&xyz_out), slice::from_mut(&mut rgb_out));

        let err = (rgb_out[0] - v)
            .abs()
            .max((rgb_out[1] - v).abs())
            .max((rgb_out[2] - v).abs());

        if err > max_error {
            max_error = err;
        }
    }

    assert!(
        max_error < 0.001,
        "Float roundtrip max error should be < 0.001, got {}",
        max_error
    );
}

/// Test white point preservation
#[test]
fn test_white_point_preservation() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // White (1,1,1) should map to D50 white point
    let white = [1.0f32, 1.0, 1.0];
    let mut xyz_out = [0.0f32; 3];
    transform.transform_pixels(slice::from_ref(&white), slice::from_mut(&mut xyz_out));

    // D50 white point: X=0.9642, Y=1.0, Z=0.8249 (normalized to Y=1)
    // But for sRGB (D65 source), with relative colorimetric intent,
    // the white point should be adapted to D50

    // Y should be close to the sum of X+Y+Z ≈ white point luminance
    let y = xyz_out[1];
    assert!((y - 1.0).abs() < 0.01, "White Y should be ~1.0, got {}", y);

    // Verify x,y chromaticity
    let sum = xyz_out[0] + xyz_out[1] + xyz_out[2];
    let x = xyz_out[0] / sum;
    let y_chrom = xyz_out[1] / sum;

    // D50 chromaticity: x≈0.3457, y≈0.3585
    assert!(
        (x - 0.3457).abs() < 0.01,
        "White x chromaticity should be ~0.3457, got {}",
        x
    );
    assert!(
        (y_chrom - 0.3585).abs() < 0.01,
        "White y chromaticity should be ~0.3585, got {}",
        y_chrom
    );
}

/// Test black point preservation
#[test]
fn test_black_point_preservation() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // Black (0,0,0) should map to XYZ (0,0,0)
    let black = [0.0f32, 0.0, 0.0];
    let mut xyz_out = [0.0f32; 3];
    transform.transform_pixels(slice::from_ref(&black), slice::from_mut(&mut xyz_out));

    assert!(
        xyz_out[0].abs() < 0.001 && xyz_out[1].abs() < 0.001 && xyz_out[2].abs() < 0.001,
        "Black should map to XYZ (0,0,0), got {:?}",
        xyz_out
    );
}

// ============================================================================
// Summary Test
// ============================================================================

#[test]
fn test_profiles_transforms_summary() {
    println!("lcms2 profiles and transforms test summary:");
    println!("  Profile creation: sRGB, XYZ, Gray, RGB");
    println!("  sRGB primaries verification");
    println!("  Known sRGB->XYZ values");
    println!("  Transform tests: identity, 8-bit, 16-bit, float");
    println!("  ICC roundtrip serialization");
    println!("  Gray profile transforms");
    println!("  Rendering intents");
    println!("  Profile version handling");
    println!("  White/black point preservation");
}
