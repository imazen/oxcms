//! Advanced lcms2 testbed tests
//!
//! Tests ported from the Little CMS testbed covering more advanced scenarios:
//! - sRGB roundtrip stability
//! - Lab profile transforms
//! - Proofing transforms
//! - Rec709 parametric curves
//!
//! Original source: https://github.com/mm2/Little-CMS/blob/master/testbed/testcms2.c

use lcms2::{CIExyY, CIExyYTRIPLE, Flags, Intent, PixelFormat, Profile, ToneCurve};
use std::slice;

// ============================================================================
// D50 White Point Constants
// ============================================================================

/// D50 white point for Lab profile creation
fn d50_white_point() -> CIExyY {
    CIExyY {
        x: 0.3457,
        y: 0.3585,
        Y: 1.0,
    }
}

// ============================================================================
// sRGB Roundtrip Stability Tests
// Port of Check_sRGB_Rountrips from testcms2.c
// ============================================================================

/// Helper function to calculate RGB distance
fn rgb_distance_16(a: &[u16; 3], b: &[u16; 3]) -> f64 {
    let dr = a[0] as f64 - b[0] as f64;
    let dg = a[1] as f64 - b[1] as f64;
    let db = a[2] as f64 - b[2] as f64;
    (dr * dr + dg * dg + db * db).sqrt()
}

/// Test sRGB roundtrip stability through Lab
/// This tests that repeated RGB -> Lab -> RGB conversions are stable
/// (a regression test for lcms2 2.12)
#[test]
fn test_srgb_lab_roundtrip_stability() {
    let srgb = Profile::new_srgb();
    let lab = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab profile creation failed");

    // sRGB -> Lab
    let forward = lcms2::Transform::<[u16; 3], [f64; 3]>::new(
        &srgb,
        PixelFormat::RGB_16,
        &lab,
        PixelFormat::Lab_DBL,
        Intent::RelativeColorimetric,
    )
    .expect("Forward transform failed");

    // Lab -> sRGB
    let backward = lcms2::Transform::<[f64; 3], [u16; 3]>::new(
        &lab,
        PixelFormat::Lab_DBL,
        &srgb,
        PixelFormat::RGB_16,
        Intent::RelativeColorimetric,
    )
    .expect("Backward transform failed");

    let mut max_err = 0.0f64;

    // Test a range of RGB values
    for r in (0u8..=255).step_by(16) {
        for g in (0u8..=255).step_by(16) {
            for b in (0u8..=255).step_by(16) {
                // Convert 8-bit to 16-bit (expand)
                let seed: [u16; 3] = [
                    ((r as u16) << 8) | (r as u16),
                    ((g as u16) << 8) | (g as u16),
                    ((b as u16) << 8) | (b as u16),
                ];
                let mut rgb = seed;

                // Perform multiple roundtrips (50 iterations as in lcms2 testbed)
                for _ in 0..50 {
                    let mut lab_val = [0.0f64; 3];
                    forward.transform_pixels(slice::from_ref(&rgb), slice::from_mut(&mut lab_val));
                    backward.transform_pixels(slice::from_ref(&lab_val), slice::from_mut(&mut rgb));
                }

                let err = rgb_distance_16(&seed, &rgb);
                if err > max_err {
                    max_err = err;
                }
            }
        }
    }

    // lcms2 testbed accepts up to 20.0
    assert!(
        max_err <= 20.0,
        "sRGB roundtrip stability error too high: {} (expected <= 20.0)",
        max_err
    );
}

/// Test sRGB to Lab float roundtrip with alpha
/// Port of ChecksRGB2LabFLT from testcms2.c
#[test]
fn test_srgb_lab_float_roundtrip() {
    let srgb = Profile::new_srgb();
    let lab = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab profile creation failed");

    // Create transforms with NOCACHE flag to test non-optimized path
    let forward = lcms2::Transform::<[f32; 4], [f32; 4]>::new_flags(
        &srgb,
        PixelFormat::RGBA_FLT,
        &lab,
        PixelFormat::LabA_FLT,
        Intent::Perceptual,
        Flags::NO_CACHE,
    )
    .expect("Forward transform failed");

    let backward = lcms2::Transform::<[f32; 4], [f32; 4]>::new_flags(
        &lab,
        PixelFormat::LabA_FLT,
        &srgb,
        PixelFormat::RGBA_FLT,
        Intent::Perceptual,
        Flags::NO_CACHE,
    )
    .expect("Backward transform failed");

    let tolerance = 0.001f32;

    // Test gray values from 0 to 1
    for i in 0..=100 {
        let v = i as f32 / 100.0;
        let rgba1 = [v, v, v, 0.0f32];

        let mut lab_val = [0.0f32; 4];
        let mut rgba2 = [0.0f32; 4];

        forward.transform_pixels(slice::from_ref(&rgba1), slice::from_mut(&mut lab_val));
        backward.transform_pixels(slice::from_ref(&lab_val), slice::from_mut(&mut rgba2));

        // Check roundtrip accuracy
        let err_r = (rgba1[0] - rgba2[0]).abs();
        let err_g = (rgba1[1] - rgba2[1]).abs();
        let err_b = (rgba1[2] - rgba2[2]).abs();

        assert!(
            err_r < tolerance && err_g < tolerance && err_b < tolerance,
            "Float RGB->Lab->RGB roundtrip failed at {}: [{:.4},{:.4},{:.4}] -> [{:.4},{:.4},{:.4}]",
            v,
            rgba1[0],
            rgba1[1],
            rgba1[2],
            rgba2[0],
            rgba2[1],
            rgba2[2]
        );
    }
}

// ============================================================================
// Proofing Transform Tests
// Port of CheckProofingXFORMFloat and CheckProofingXFORM16 from testcms2.c
// ============================================================================

/// Test proofing transform with float values
#[test]
fn test_proofing_transform_float() {
    let srgb = Profile::new_srgb();

    // Create a proofing transform where sRGB proofs to itself
    // This should be effectively an identity transform
    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new_proofing(
        &srgb,
        PixelFormat::RGB_FLT,
        &srgb,
        PixelFormat::RGB_FLT,
        &srgb,
        Intent::RelativeColorimetric,
        Intent::RelativeColorimetric,
        Flags::SOFT_PROOFING,
    )
    .expect("Proofing transform creation failed");

    // Test that it approximates identity
    for i in 0..=10 {
        let v = i as f32 / 10.0;
        let input = [v, v, v];
        let mut output = [0.0f32; 3];

        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        let max_err = (output[0] - v)
            .abs()
            .max((output[1] - v).abs())
            .max((output[2] - v).abs());

        assert!(
            max_err < 0.01,
            "Proofing transform not identity at {}: got {:?}, error {}",
            v,
            output,
            max_err
        );
    }
}

/// Test proofing transform with 16-bit values
#[test]
fn test_proofing_transform_16bit() {
    let srgb = Profile::new_srgb();

    // Create a proofing transform where sRGB proofs to itself
    let transform = lcms2::Transform::<[u16; 3], [u16; 3]>::new_proofing(
        &srgb,
        PixelFormat::RGB_16,
        &srgb,
        PixelFormat::RGB_16,
        &srgb,
        Intent::RelativeColorimetric,
        Intent::RelativeColorimetric,
        Flags::SOFT_PROOFING,
    )
    .expect("Proofing transform creation failed");

    // Test at several 16-bit values
    let test_values: [u16; 5] = [0, 16384, 32768, 49152, 65535];

    for &v in &test_values {
        let input = [v, v, v];
        let mut output = [0u16; 3];

        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        // Allow some tolerance for 16-bit proofing
        let max_err = (output[0] as i32 - v as i32)
            .abs()
            .max((output[1] as i32 - v as i32).abs())
            .max((output[2] as i32 - v as i32).abs());

        assert!(
            max_err <= 512,
            "16-bit proofing at {}: got {:?}, error {}",
            v,
            output,
            max_err
        );
    }
}

// ============================================================================
// Gamut Check Tests
// Port of CheckGamutCheck from testcms2.c
// ============================================================================

/// Test gamut checking functionality
#[test]
fn test_gamut_check_same_profile() {
    let srgb = Profile::new_srgb();

    // Set alarm codes for out-of-gamut colors (needs 16 channels)
    let alarm_codes: [u16; 16] = [0xDEAD, 0xBABE, 0xFACE, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    #[allow(deprecated)]
    lcms2::Transform::<[f32; 3], [f32; 3]>::set_global_alarm_codes(alarm_codes);

    // Create a gamut checking transform where same profile checks against itself
    // No values should be out of gamut
    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new_proofing(
        &srgb,
        PixelFormat::RGB_FLT,
        &srgb,
        PixelFormat::RGB_FLT,
        &srgb,
        Intent::RelativeColorimetric,
        Intent::RelativeColorimetric,
        Flags::GAMUT_CHECK,
    )
    .expect("Gamut check transform creation failed");

    // Test that values pass through (none should trigger gamut alarm)
    for i in 0..=10 {
        let v = i as f32 / 10.0;
        let input = [v, v, v];
        let mut output = [0.0f32; 3];

        transform.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut output));

        let max_err = (output[0] - v)
            .abs()
            .max((output[1] - v).abs())
            .max((output[2] - v).abs());

        assert!(
            max_err < 0.01,
            "Gamut check should not trigger for in-gamut colors at {}: got {:?}",
            v,
            output
        );
    }
}

// ============================================================================
// Rec709 Parametric Curve Tests
// Port of CheckParametricRec709 from testcms2.c
// ============================================================================

/// Rec709 transfer function (linear segment + power curve)
fn rec709(l: f64) -> f64 {
    if l < 0.018 {
        4.5 * l
    } else {
        1.099 * l.powf(0.45) - 0.099
    }
}

/// Test Rec709 parametric curve (type 5)
/// Y = (aX + b)^Gamma + e  if X >= d
/// Y = cX + f              if X < d
#[test]
fn test_parametric_curve_rec709() {
    // Rec709 parameters:
    // gamma = 0.45
    // a = 1.099^(1/0.45) ≈ 4.5
    // b = 0
    // c = 4.5
    // d = 0.018
    // e = -0.099
    // f = 0
    let gamma = 0.45;
    let a = 1.099f64.powf(1.0 / 0.45);
    let b = 0.0;
    let c = 4.5;
    let d = 0.018;
    let e = -0.099;
    let f = 0.0;

    let params = [gamma, a, b, c, d, e, f];
    let curve = ToneCurve::new_parametric(5, &params).expect("Rec709 curve creation failed");

    // Test at several points
    let test_points = [0.0, 0.01, 0.018, 0.05, 0.1, 0.5, 1.0];

    for &x in &test_points {
        let expected = rec709(x);
        let actual = curve.eval(x as f32) as f64;

        let err = (expected - actual).abs();
        assert!(
            err < 0.01,
            "Rec709 curve at {}: expected {:.6}, got {:.6}, error {:.6}",
            x,
            expected,
            actual,
            err
        );
    }
}

// ============================================================================
// Lab Profile Tests
// ============================================================================

/// Test Lab V2 profile creation and transform
#[test]
fn test_lab_v2_profile() {
    let lab2 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile creation failed");

    assert_eq!(
        lab2.color_space(),
        lcms2::ColorSpaceSignature::LabData,
        "Lab V2 should have Lab color space"
    );

    // Version should be 2.x
    let version = lab2.version();
    assert!(
        version >= 2.0 && version < 3.0,
        "Lab V2 should be version 2.x, got {}",
        version
    );
}

/// Test Lab V4 profile creation and transform
#[test]
fn test_lab_v4_profile() {
    let lab4 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile creation failed");

    assert_eq!(
        lab4.color_space(),
        lcms2::ColorSpaceSignature::LabData,
        "Lab V4 should have Lab color space"
    );

    // Version should be 4.x
    let version = lab4.version();
    assert!(
        version >= 4.0 && version < 5.0,
        "Lab V4 should be version 4.x, got {}",
        version
    );
}

/// Test Lab identity transform
#[test]
fn test_lab_identity_transform() {
    let lab = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab profile creation failed");

    // Lab -> Lab should be identity
    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &lab,
        PixelFormat::Lab_FLT,
        &lab,
        PixelFormat::Lab_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // Test several Lab values
    let test_cases = [
        [0.0f32, 0.0, 0.0],    // Black
        [100.0, 0.0, 0.0],     // White
        [50.0, 0.0, 0.0],      // Mid-gray
        [50.0, 50.0, 0.0],     // Reddish
        [50.0, -50.0, 0.0],    // Greenish
        [50.0, 0.0, 50.0],     // Yellowish
        [50.0, 0.0, -50.0],    // Bluish
        [75.0, 25.0, -25.0],   // Arbitrary
    ];

    for lab_in in test_cases {
        let mut lab_out = [0.0f32; 3];
        transform.transform_pixels(slice::from_ref(&lab_in), slice::from_mut(&mut lab_out));

        let max_err = (lab_out[0] - lab_in[0])
            .abs()
            .max((lab_out[1] - lab_in[1]).abs())
            .max((lab_out[2] - lab_in[2]).abs());

        assert!(
            max_err < 0.001,
            "Lab identity failed: {:?} -> {:?}, error {}",
            lab_in,
            lab_out,
            max_err
        );
    }
}

// ============================================================================
// Custom RGB Profile Tests
// ============================================================================

/// Test creating custom RGB profile with Rec709 primaries
#[test]
fn test_custom_rgb_rec709() {
    // D65 white point
    let white = CIExyY {
        x: 0.3127,
        y: 0.3290,
        Y: 1.0,
    };

    // Rec709 primaries
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

    // Use gamma 2.4 (Rec709-ish)
    let gamma = ToneCurve::new(2.4);
    let curves = [&gamma, &gamma, &gamma];

    let rec709 =
        Profile::new_rgb(&white, &primaries, &curves).expect("Rec709 profile creation failed");

    assert_eq!(
        rec709.color_space(),
        lcms2::ColorSpaceSignature::RgbData,
        "Rec709 should have RGB color space"
    );

    // Transform from Rec709 to sRGB and back should approximately preserve colors
    let srgb = Profile::new_srgb();

    let forward = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &rec709,
        PixelFormat::RGB_FLT,
        &srgb,
        PixelFormat::RGB_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Forward transform failed");

    let backward = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &rec709,
        PixelFormat::RGB_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Backward transform failed");

    // Test roundtrip for gray values
    for i in 0..=10 {
        let v = i as f32 / 10.0;
        let input = [v, v, v];
        let mut mid = [0.0f32; 3];
        let mut output = [0.0f32; 3];

        forward.transform_pixels(slice::from_ref(&input), slice::from_mut(&mut mid));
        backward.transform_pixels(slice::from_ref(&mid), slice::from_mut(&mut output));

        // Gray should be mostly preserved (primaries are similar)
        let max_err = (output[0] - v)
            .abs()
            .max((output[1] - v).abs())
            .max((output[2] - v).abs());

        assert!(
            max_err < 0.02,
            "Rec709<->sRGB roundtrip error at {}: got {:?}, error {}",
            v,
            output,
            max_err
        );
    }
}

// ============================================================================
// Multi-Profile Transform Tests
// ============================================================================

/// Test transform with device link profile
#[test]
fn test_device_link_creation() {
    let srgb = Profile::new_srgb();
    let xyz = Profile::new_xyz();

    // Create a simple sRGB to XYZ transform
    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &srgb,
        PixelFormat::RGB_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // Create a device link from this transform
    let link = Profile::new_device_link(&transform, 4.3, Flags::default())
        .expect("Device link creation failed");

    assert_eq!(
        link.device_class(),
        lcms2::ProfileClassSignature::LinkClass,
        "Device link should have Link class"
    );

    // Device link should have RGB as input color space
    assert_eq!(
        link.color_space(),
        lcms2::ColorSpaceSignature::RgbData,
        "Device link input should be RGB"
    );

    // Device link PCS should be XYZ (the output)
    assert_eq!(
        link.pcs(),
        lcms2::ColorSpaceSignature::XYZData,
        "Device link output should be XYZ"
    );

    // Serialize to ICC and verify it's valid
    let icc = link.icc().expect("Device link ICC serialization failed");
    assert!(
        icc.len() > 100,
        "Device link ICC should have reasonable size"
    );

    // Reload the device link and verify properties
    let reloaded = Profile::new_icc(&icc).expect("Device link ICC reload failed");
    assert_eq!(
        reloaded.device_class(),
        lcms2::ProfileClassSignature::LinkClass,
        "Reloaded device link should have Link class"
    );
}

// ============================================================================
// Lab V2/V4 Cross-Version Transform Tests
// Port of CheckFloatLabTransforms from testcms2.c
// ============================================================================

/// Helper function to check Lab identity transform
fn check_lab_identity(lab1: &Profile, lab2: &Profile, name: &str) {
    let transform = lcms2::Transform::<[f64; 3], [f64; 3]>::new(
        lab1,
        PixelFormat::Lab_DBL,
        lab2,
        PixelFormat::Lab_DBL,
        Intent::RelativeColorimetric,
    )
    .expect(&format!("{} transform creation failed", name));

    // Test several Lab values
    let test_cases = [
        [0.0f64, 0.0, 0.0],     // Black
        [100.0, 0.0, 0.0],      // White
        [50.0, 0.0, 0.0],       // Mid-gray
        [50.0, 50.0, 0.0],      // Reddish
        [50.0, -50.0, 0.0],     // Greenish
        [50.0, 0.0, 50.0],      // Yellowish
        [50.0, 0.0, -50.0],     // Bluish
        [75.0, 25.0, -25.0],    // Arbitrary color
        [25.0, -30.0, 40.0],    // Another arbitrary
    ];

    for lab_in in test_cases {
        let mut lab_out = [0.0f64; 3];
        transform.transform_pixels(slice::from_ref(&lab_in), slice::from_mut(&mut lab_out));

        let max_err = (lab_out[0] - lab_in[0])
            .abs()
            .max((lab_out[1] - lab_in[1]).abs())
            .max((lab_out[2] - lab_in[2]).abs());

        assert!(
            max_err < 0.001,
            "{} identity failed: {:?} -> {:?}, error {}",
            name,
            lab_in,
            lab_out,
            max_err
        );
    }
}

/// Test Lab V4 to Lab V4 transform (identity)
#[test]
fn test_lab_v4_to_v4_transform() {
    let lab1 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile 1 creation failed");
    let lab2 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile 2 creation failed");

    check_lab_identity(&lab1, &lab2, "Lab4/Lab4");
}

/// Test Lab V2 to Lab V2 transform (identity)
#[test]
fn test_lab_v2_to_v2_transform() {
    let lab1 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile 1 creation failed");
    let lab2 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile 2 creation failed");

    check_lab_identity(&lab1, &lab2, "Lab2/Lab2");
}

/// Test Lab V4 to Lab V2 transform (cross-version)
#[test]
fn test_lab_v4_to_v2_transform() {
    let lab_v4 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile creation failed");
    let lab_v2 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile creation failed");

    check_lab_identity(&lab_v4, &lab_v2, "Lab4/Lab2");
}

/// Test Lab V2 to Lab V4 transform (cross-version)
#[test]
fn test_lab_v2_to_v4_transform() {
    let lab_v2 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile creation failed");
    let lab_v4 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile creation failed");

    check_lab_identity(&lab_v2, &lab_v4, "Lab2/Lab4");
}

// ============================================================================
// Lab Encoded Transform Tests
// Port of CheckEncodedLabTransforms from testcms2.c
// ============================================================================

/// Test encoded Lab V4 to float Lab transform
#[test]
fn test_encoded_lab_v4_transform() {
    let lab1 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile 1 creation failed");
    let lab2 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile 2 creation failed");

    // Lab16 to Lab_DBL
    let transform = lcms2::Transform::<[u16; 3], [f64; 3]>::new(
        &lab1,
        PixelFormat::Lab_16,
        &lab2,
        PixelFormat::Lab_DBL,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // Test white: 0xFFFF, 0x8080, 0x8080 should give L=100, a=0, b=0
    let white_in: [u16; 3] = [0xFFFF, 0x8080, 0x8080];
    let mut lab_out = [0.0f64; 3];
    transform.transform_pixels(slice::from_ref(&white_in), slice::from_mut(&mut lab_out));

    let white_expected = [100.0f64, 0.0, 0.0];
    let err_l = (lab_out[0] - white_expected[0]).abs();
    let err_a = (lab_out[1] - white_expected[1]).abs();
    let err_b = (lab_out[2] - white_expected[2]).abs();

    assert!(
        err_l < 0.01 && err_a < 0.01 && err_b < 0.01,
        "Encoded white Lab16 -> Lab_DBL failed: {:?} -> {:?}",
        white_in,
        lab_out
    );

    // Test a color value
    let color_in: [u16; 3] = [0x1234, 0x3434, 0x9A9A];
    transform.transform_pixels(slice::from_ref(&color_in), slice::from_mut(&mut lab_out));

    // The expected color is L=7.11070, a=-76, b=26 from lcms2 testbed
    let color_expected = [7.11070f64, -76.0, 26.0];
    let err_l = (lab_out[0] - color_expected[0]).abs();
    let err_a = (lab_out[1] - color_expected[1]).abs();
    let err_b = (lab_out[2] - color_expected[2]).abs();

    // Use larger tolerance for this specific color
    assert!(
        err_l < 0.1 && err_a < 1.0 && err_b < 1.0,
        "Encoded color Lab16 -> Lab_DBL failed: {:?} -> L={:.5} a={:.2} b={:.2} (expected L={:.5} a={:.0} b={:.0})",
        color_in,
        lab_out[0],
        lab_out[1],
        lab_out[2],
        color_expected[0],
        color_expected[1],
        color_expected[2]
    );
}

/// Test Lab V2 encoding to Lab V4
#[test]
fn test_lab_v2_to_v4_encoded() {
    let lab_v2 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile creation failed");
    let lab_v4 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile creation failed");

    // Lab V2 uses different encoding (0xFF00 for L=100)
    let transform = lcms2::Transform::<[u16; 3], [f64; 3]>::new(
        &lab_v2,
        PixelFormat::LabV2_16,
        &lab_v4,
        PixelFormat::Lab_DBL,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // V2 white: 0xFF00, 0x8000, 0x8000
    let white_in: [u16; 3] = [0xFF00, 0x8000, 0x8000];
    let mut lab_out = [0.0f64; 3];
    transform.transform_pixels(slice::from_ref(&white_in), slice::from_mut(&mut lab_out));

    let err_l = (lab_out[0] - 100.0).abs();
    let err_a = lab_out[1].abs();
    let err_b = lab_out[2].abs();

    assert!(
        err_l < 0.1 && err_a < 0.1 && err_b < 0.1,
        "Lab V2 white encoding failed: {:?} -> {:?}",
        white_in,
        lab_out
    );
}

/// Test Lab V4 to Lab V2 encoding
#[test]
fn test_lab_v4_to_v2_encoded() {
    let lab_v4 = Profile::new_lab4_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V4 profile creation failed");
    let lab_v2 = Profile::new_lab2_context(lcms2::GlobalContext::new(), &d50_white_point())
        .expect("Lab V2 profile creation failed");

    // Lab_DBL to Lab V2
    let transform = lcms2::Transform::<[f64; 3], [u16; 3]>::new(
        &lab_v4,
        PixelFormat::Lab_DBL,
        &lab_v2,
        PixelFormat::LabV2_16,
        Intent::RelativeColorimetric,
    )
    .expect("Transform creation failed");

    // White in Lab_DBL
    let white_in: [f64; 3] = [100.0, 0.0, 0.0];
    let mut v2_out = [0u16; 3];
    transform.transform_pixels(slice::from_ref(&white_in), slice::from_mut(&mut v2_out));

    // V2 white should be 0xFF00, 0x8000, 0x8000
    assert_eq!(
        v2_out[0], 0xFF00,
        "Lab V4 white -> V2 L encoding failed: got 0x{:04X}",
        v2_out[0]
    );
    assert_eq!(
        v2_out[1], 0x8000,
        "Lab V4 white -> V2 a encoding failed: got 0x{:04X}",
        v2_out[1]
    );
    assert_eq!(
        v2_out[2], 0x8000,
        "Lab V4 white -> V2 b encoding failed: got 0x{:04X}",
        v2_out[2]
    );
}

// ============================================================================
// XYZ Float Transform Tests
// ============================================================================

/// Test XYZ to XYZ float transform (identity)
#[test]
fn test_xyz_identity_float() {
    let xyz = Profile::new_xyz();

    let transform = lcms2::Transform::<[f32; 3], [f32; 3]>::new(
        &xyz,
        PixelFormat::XYZ_FLT,
        &xyz,
        PixelFormat::XYZ_FLT,
        Intent::RelativeColorimetric,
    )
    .expect("XYZ identity transform creation failed");

    // Test D50 white point
    let d50_xyz = [0.9642f32, 1.0, 0.8249];
    let mut xyz_out = [0.0f32; 3];
    transform.transform_pixels(slice::from_ref(&d50_xyz), slice::from_mut(&mut xyz_out));

    let max_err = (xyz_out[0] - d50_xyz[0])
        .abs()
        .max((xyz_out[1] - d50_xyz[1]).abs())
        .max((xyz_out[2] - d50_xyz[2]).abs());

    assert!(
        max_err < 0.001,
        "XYZ identity D50 failed: {:?} -> {:?}, error {}",
        d50_xyz,
        xyz_out,
        max_err
    );

    // Test arbitrary XYZ values
    let test_values = [
        [0.0f32, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];

    for xyz_in in test_values {
        transform.transform_pixels(slice::from_ref(&xyz_in), slice::from_mut(&mut xyz_out));

        let max_err = (xyz_out[0] - xyz_in[0])
            .abs()
            .max((xyz_out[1] - xyz_in[1]).abs())
            .max((xyz_out[2] - xyz_in[2]).abs());

        assert!(
            max_err < 0.001,
            "XYZ identity failed: {:?} -> {:?}, error {}",
            xyz_in,
            xyz_out,
            max_err
        );
    }
}

// ============================================================================
// Summary Test
// ============================================================================

#[test]
fn test_advanced_summary() {
    println!("lcms2 advanced tests summary:");
    println!("  - sRGB roundtrip stability");
    println!("  - sRGB->Lab float roundtrip");
    println!("  - Proofing transforms (float, 16-bit)");
    println!("  - Gamut checking");
    println!("  - Rec709 parametric curve");
    println!("  - Lab V2/V4 profiles and cross-version transforms");
    println!("  - Lab encoded transforms (V2/V4)");
    println!("  - XYZ identity transforms");
    println!("  - Custom RGB profiles (Rec709)");
    println!("  - Device link profiles");
}
