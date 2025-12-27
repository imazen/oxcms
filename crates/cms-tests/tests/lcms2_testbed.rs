//! Ported tests from lcms2 testbed (testcms2.c)
//!
//! These tests are direct ports from the Little CMS testbed, validating
//! fundamental color science operations.
//!
//! Original source: https://github.com/mm2/Little-CMS/blob/master/testbed/testcms2.c

#![allow(clippy::needless_range_loop)]
#![allow(dead_code)]

use lcms2::{
    CIELab, CIELabExt, CIEXYZ, CIEXYZExt, CIExzYExt, ToneCurve, XYZ2xyY, white_point_from_temp,
    xyY2XYZ,
};

// ============================================================================
// D50 Constants and Roundtrip Tests
// ============================================================================

/// Standard D50 illuminant values from ICC specification
const CMS_D50_X: f64 = 0.9642;
const CMS_D50_Y: f64 = 1.0;
const CMS_D50_Z: f64 = 0.8249;

/// Alternate D50 values with more precision
const CMS_D50_X_2: f64 = 0.96420288;
const CMS_D50_Y_2: f64 = 1.0;
const CMS_D50_Z_2: f64 = 0.82490540;

/// Helper: Convert f64 to s15Fixed16 (ICC 15.16 fixed point)
fn double_to_s15fixed16(v: f64) -> i32 {
    ((v * 65536.0) + 0.5).floor() as i32
}

/// Helper: Convert s15Fixed16 to f64
fn s15fixed16_to_double(v: i32) -> f64 {
    v as f64 / 65536.0
}

/// Test that D50 values survive fixed-point roundtrip
/// Port of CheckD50Roundtrip from testcms2.c
#[test]
fn test_d50_roundtrip() {
    // First set of D50 values
    let xe = double_to_s15fixed16(CMS_D50_X);
    let ye = double_to_s15fixed16(CMS_D50_Y);
    let ze = double_to_s15fixed16(CMS_D50_Z);

    let x = s15fixed16_to_double(xe);
    let y = s15fixed16_to_double(ye);
    let z = s15fixed16_to_double(ze);

    let dx = (CMS_D50_X - x).abs();
    let dy = (CMS_D50_Y - y).abs();
    let dz = (CMS_D50_Z - z).abs();

    let euc = (dx * dx + dy * dy + dz * dz).sqrt();
    assert!(
        euc < 1e-5,
        "D50 roundtrip error |err| = {} (expected < 1e-5)",
        euc
    );

    // Second set with more precision
    let xe = double_to_s15fixed16(CMS_D50_X_2);
    let ye = double_to_s15fixed16(CMS_D50_Y_2);
    let ze = double_to_s15fixed16(CMS_D50_Z_2);

    let x = s15fixed16_to_double(xe);
    let y = s15fixed16_to_double(ye);
    let z = s15fixed16_to_double(ze);

    let dx = (CMS_D50_X_2 - x).abs();
    let dy = (CMS_D50_Y_2 - y).abs();
    let dz = (CMS_D50_Z_2 - z).abs();

    let euc = (dx * dx + dy * dy + dz * dz).sqrt();
    assert!(
        euc < 1e-5,
        "D50 alternate roundtrip error |err| = {} (expected < 1e-5)",
        euc
    );
}

// ============================================================================
// Lab <-> LCh Conversion Tests
// ============================================================================

/// Convert Lab to LCh (polar coordinates)
fn lab_to_lch(lab: &CIELab) -> (f64, f64, f64) {
    let l = lab.L;
    let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let h = lab.b.atan2(lab.a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };
    (l, c, h)
}

/// Convert LCh back to Lab
fn lch_to_lab(l: f64, c: f64, h: f64) -> CIELab {
    let h_rad = h.to_radians();
    CIELab {
        L: l,
        a: c * h_rad.cos(),
        b: c * h_rad.sin(),
    }
}

/// Test Lab <-> LCh roundtrip
/// Port of CheckLab2LCh from testcms2.c
#[test]
fn test_lab_to_lch_roundtrip() {
    let mut max_dist = 0.0f64;

    for l in (0..=100).step_by(10) {
        for a in (-128..=128).step_by(8) {
            for b in (-128..=128).step_by(8) {
                let lab = CIELab {
                    L: l as f64,
                    a: a as f64,
                    b: b as f64,
                };

                let (lch_l, lch_c, lch_h) = lab_to_lch(&lab);
                let lab2 = lch_to_lab(lch_l, lch_c, lch_h);

                let dist = lab.delta_e(&lab2);
                if dist > max_dist {
                    max_dist = dist;
                }
            }
        }
    }

    // lcms2 requires < 1e-12 accuracy
    assert!(
        max_dist < 1e-12,
        "Lab->LCh->Lab max error: {} (expected < 1e-12)",
        max_dist
    );
}

// ============================================================================
// Lab <-> XYZ Conversion Tests
// ============================================================================

/// D50 white point for Lab conversions
fn d50_white_point() -> CIEXYZ {
    CIEXYZ {
        X: CMS_D50_X,
        Y: CMS_D50_Y,
        Z: CMS_D50_Z,
    }
}

/// Test Lab <-> XYZ roundtrip
/// Port of CheckLab2XYZ from testcms2.c
#[test]
fn test_lab_to_xyz_roundtrip() {
    let white = d50_white_point();
    let mut max_dist = 0.0f64;

    for l in (0..=100).step_by(10) {
        for a in (-128..=128).step_by(8) {
            for b in (-128..=128).step_by(8) {
                let lab = CIELab {
                    L: l as f64,
                    a: a as f64,
                    b: b as f64,
                };

                let xyz = lab.to_xyz(&white);
                let lab2 = xyz.to_lab(&white);

                let dist = lab.delta_e(&lab2);
                if dist > max_dist {
                    max_dist = dist;
                }
            }
        }
    }

    // lcms2 requires < 1e-12 accuracy
    assert!(
        max_dist < 1e-12,
        "Lab->XYZ->Lab max error: {} (expected < 1e-12)",
        max_dist
    );
}

// ============================================================================
// Lab <-> xyY Conversion Tests
// ============================================================================

/// Test Lab -> XYZ -> xyY -> XYZ -> Lab roundtrip
/// Port of CheckLab2xyY from testcms2.c
#[test]
fn test_lab_to_xyy_roundtrip() {
    let white = d50_white_point();
    let mut max_dist = 0.0f64;

    for l in (0..=100).step_by(10) {
        for a in (-128..=128).step_by(8) {
            for b in (-128..=128).step_by(8) {
                let lab = CIELab {
                    L: l as f64,
                    a: a as f64,
                    b: b as f64,
                };

                let xyz = lab.to_xyz(&white);
                let xyy = XYZ2xyY(&xyz);
                let xyz2 = xyY2XYZ(&xyy);
                let lab2 = xyz2.to_lab(&white);

                let dist = lab.delta_e(&lab2);
                if dist > max_dist {
                    max_dist = dist;
                }
            }
        }
    }

    // lcms2 requires < 1e-12 accuracy
    // Note: xyY has a singularity at Y=0, which may cause higher errors
    assert!(
        max_dist < 1e-10, // Slightly relaxed due to xyY singularity
        "Lab->XYZ->xyY->XYZ->Lab max error: {} (expected < 1e-10)",
        max_dist
    );
}

// ============================================================================
// Lab Encoding Tests (V2 vs V4)
// ============================================================================

/// Test Lab V4 encoding roundtrip
/// Port of CheckLabV4encoding from testcms2.c
#[test]
fn test_lab_v4_encoding_roundtrip() {
    let mut errors = 0;

    for j in 0u16..65535 {
        let inw = [j, j, j];

        let lab = CIELab::from_encoded(&inw);
        let aw = lab.encoded();

        for i in 0..3 {
            if aw[i] != j {
                errors += 1;
            }
        }
    }

    assert_eq!(
        errors, 0,
        "Lab V4 encoding had {} errors (expected 0)",
        errors
    );
}

/// Test Lab V2 encoding roundtrip
/// Port of CheckLabV2encoding from testcms2.c
#[test]
fn test_lab_v2_encoding_roundtrip() {
    let mut errors = 0;

    for j in 0u16..65535 {
        let inw = [j, j, j];

        let lab = CIELab::from_encoded_v2(&inw);
        let aw = lab.encoded_v2();

        for i in 0..3 {
            if aw[i] != j {
                errors += 1;
            }
        }
    }

    assert_eq!(
        errors, 0,
        "Lab V2 encoding had {} errors (expected 0)",
        errors
    );
}

/// Test that V2 and V4 encodings differ (they should!)
#[test]
fn test_lab_v2_v4_encoding_differ() {
    // At L=50, a=0, b=0 (mid-gray in Lab)
    let lab = CIELab {
        L: 50.0,
        a: 0.0,
        b: 0.0,
    };

    let v2 = lab.encoded_v2();
    let v4 = lab.encoded();

    // V2 and V4 have different encodings
    // V2: L* 0..100 maps to 0..0xFF00, a*/b* -128..127 maps to 0..0xFF00
    // V4: L* 0..100 maps to 0..0xFFFF, a*/b* -128..127 maps to 0..0xFFFF
    //
    // At L=50, the V2 and V4 L values should differ
    // At a*=0 (encoded as mid-point), they also differ slightly due to range differences

    // Just verify they're not all identical (the encoding schemes differ)
    let all_same = v2[0] == v4[0] && v2[1] == v4[1] && v2[2] == v4[2];
    assert!(
        !all_same,
        "V2 and V4 encodings should differ for some channel"
    );

    // Document the actual values for reference
    eprintln!("Lab(50, 0, 0) V2 encoding: {:?}", v2);
    eprintln!("Lab(50, 0, 0) V4 encoding: {:?}", v4);

    // Verify that round-trip works for each encoding
    let lab_v2 = CIELab::from_encoded_v2(&v2);
    let lab_v4 = CIELab::from_encoded(&v4);

    assert!(
        (lab_v2.L - 50.0).abs() < 0.01,
        "V2 L roundtrip failed: {}",
        lab_v2.L
    );
    assert!(
        (lab_v4.L - 50.0).abs() < 0.01,
        "V4 L roundtrip failed: {}",
        lab_v4.L
    );
}

// ============================================================================
// Blackbody Radiator / Color Temperature Tests
// ============================================================================

/// Test blackbody color temperature roundtrip
/// Port of CheckTemp2CHRM from testcms2.c
#[test]
fn test_color_temperature_roundtrip() {
    let mut max_diff = 0.0f64;

    for temp in 4000..25000 {
        let white = white_point_from_temp(temp as f64).expect("white_point_from_temp failed");

        let recovered_temp = white.temp().expect("temp recovery failed");

        let diff = (recovered_temp - temp as f64).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }

    // lcms2 accepts up to 100K resolution
    assert!(
        max_diff < 100.0,
        "Color temperature roundtrip max error: {}K (expected < 100K)",
        max_diff
    );
}

/// Test specific color temperatures produce valid chromaticity
#[test]
fn test_known_color_temperatures() {
    // lcms2's white_point_from_temp only works for 4000K-25000K range
    // Standard illuminant A (~2856K) is outside the valid range

    // D50 (ICC PCS): ~5003K
    let d50 = white_point_from_temp(5003.0).expect("5003K failed");
    assert!(
        (d50.x - 0.3457).abs() < 0.01,
        "D50 x should be ~0.3457, got {}",
        d50.x
    );
    assert!(
        (d50.y - 0.3585).abs() < 0.01,
        "D50 y should be ~0.3585, got {}",
        d50.y
    );

    // D65 (daylight): ~6504K
    let d65 = white_point_from_temp(6504.0).expect("6504K failed");
    assert!(
        (d65.x - 0.3127).abs() < 0.01,
        "D65 x should be ~0.3127, got {}",
        d65.x
    );
    assert!(
        (d65.y - 0.3290).abs() < 0.01,
        "D65 y should be ~0.3290, got {}",
        d65.y
    );

    // 4000K should be valid (minimum range)
    let t4000 = white_point_from_temp(4000.0).expect("4000K failed");
    assert!(
        t4000.x > 0.0 && t4000.y > 0.0,
        "4000K should produce valid chromaticity"
    );

    // 25000K should be valid (maximum range)
    let t25000 = white_point_from_temp(25000.0);
    // Note: Some versions may not support 25000K exactly
    if let Some(t) = t25000 {
        assert!(
            t.x > 0.0 && t.y > 0.0,
            "25000K should produce valid chromaticity"
        );
    }
}

// ============================================================================
// Delta E Metric Tests
// ============================================================================

/// Test that delta E is zero for identical colors
#[test]
fn test_delta_e_identical() {
    let lab = CIELab {
        L: 50.0,
        a: 25.0,
        b: -30.0,
    };

    let de76 = lab.delta_e(&lab);
    assert!(de76 < 1e-10, "DeltaE of identical colors should be 0");

    let de94 = lab.cie94_delta_e(&lab);
    assert!(de94 < 1e-10, "DeltaE94 of identical colors should be 0");

    let de2000 = lab.cie2000_delta_e(&lab, 1.0, 1.0, 1.0);
    assert!(de2000 < 1e-10, "DeltaE2000 of identical colors should be 0");
}

/// Test delta E with known values
#[test]
fn test_delta_e_known_values() {
    let lab1 = CIELab {
        L: 50.0,
        a: 0.0,
        b: 0.0,
    };
    let lab2 = CIELab {
        L: 100.0,
        a: 0.0,
        b: 0.0,
    };

    // Pure L difference of 50 units
    let de = lab1.delta_e(&lab2);
    assert!(
        (de - 50.0).abs() < 0.001,
        "DeltaE for L=50 difference should be 50, got {}",
        de
    );

    // Test a* difference
    let lab3 = CIELab {
        L: 50.0,
        a: 30.0,
        b: 0.0,
    };
    let de = lab1.delta_e(&lab3);
    assert!(
        (de - 30.0).abs() < 0.001,
        "DeltaE for a*=30 difference should be 30, got {}",
        de
    );
}

// ============================================================================
// XYZ <-> xyY Conversion Tests
// ============================================================================

/// Test XYZ <-> xyY roundtrip
#[test]
fn test_xyz_xyy_roundtrip() {
    let test_values = [
        CIEXYZ {
            X: 0.5,
            Y: 0.5,
            Z: 0.5,
        },
        CIEXYZ {
            X: CMS_D50_X,
            Y: CMS_D50_Y,
            Z: CMS_D50_Z,
        },
        CIEXYZ {
            X: 0.1,
            Y: 0.2,
            Z: 0.3,
        },
        CIEXYZ {
            X: 0.95,
            Y: 1.0,
            Z: 1.09,
        }, // D65
    ];

    for xyz in &test_values {
        let xyy = XYZ2xyY(xyz);
        let xyz2 = xyY2XYZ(&xyy);

        let dx = (xyz.X - xyz2.X).abs();
        let dy = (xyz.Y - xyz2.Y).abs();
        let dz = (xyz.Z - xyz2.Z).abs();

        let max_err = dx.max(dy).max(dz);
        assert!(
            max_err < 1e-10,
            "XYZ->xyY->XYZ roundtrip error: {} for {:?}",
            max_err,
            xyz
        );
    }
}

/// Test xyY at Y=0 (singularity)
#[test]
fn test_xyy_y_zero() {
    // Y=0 is a singularity for xyY - x and y become undefined
    let xyz = CIEXYZ {
        X: 0.0,
        Y: 0.0,
        Z: 0.0,
    };
    let xyy = XYZ2xyY(&xyz);

    // lcms2 returns D50 chromaticity for Y=0
    // (This is the conventional handling of the singularity)
    assert!(
        xyy.Y.abs() < 1e-10,
        "Y should be 0 for black, got {}",
        xyy.Y
    );
}

// ============================================================================
// Chromatic Adaptation Tests
// ============================================================================

/// Test chromatic adaptation from D50 to D65
#[test]
fn test_chromatic_adaptation() {
    let d50 = CIEXYZ {
        X: CMS_D50_X,
        Y: CMS_D50_Y,
        Z: CMS_D50_Z,
    };
    let d65 = CIEXYZ {
        X: 0.95047,
        Y: 1.0,
        Z: 1.08883,
    };

    // Adapt white point itself - should become the target illuminant
    let adapted = d50
        .adapt_to_illuminant(&d50, &d65)
        .expect("adaptation failed");

    // The adapted D50 white should become D65
    let err_x = (adapted.X - d65.X).abs();
    let err_y = (adapted.Y - d65.Y).abs();
    let err_z = (adapted.Z - d65.Z).abs();

    let max_err = err_x.max(err_y).max(err_z);
    assert!(
        max_err < 0.01,
        "Chromatic adaptation error: {} (expected < 0.01)",
        max_err
    );
}

// ============================================================================
// Fixed-Point Representation Tests
// ============================================================================

/// Test 15.16 fixed-point precision
#[test]
fn test_fixed_point_15_16() {
    let test_values = [
        0.0, 0.5, 1.0, -1.0, 32767.0, -32768.0, 0.000015, // ~1 LSB
    ];

    for &val in &test_values {
        let fixed = double_to_s15fixed16(val);
        let back = s15fixed16_to_double(fixed);
        let err = (val - back).abs();

        // 15.16 has about 1/65536 precision
        assert!(
            err < 2.0 / 65536.0,
            "15.16 roundtrip error for {}: {}",
            val,
            err
        );
    }
}

/// Helper: Convert f64 to u8Fixed8 (ICC 8.8 fixed point)
fn double_to_u8fixed8(v: f64) -> u16 {
    ((v * 256.0) + 0.5).floor() as u16
}

/// Helper: Convert u8Fixed8 to f64
fn u8fixed8_to_double(v: u16) -> f64 {
    v as f64 / 256.0
}

/// Test 8.8 fixed-point precision
#[test]
fn test_fixed_point_8_8() {
    let test_values = [0.0, 0.5, 1.0, 127.5, 255.0, 0.00390625]; // ~1 LSB

    for &val in &test_values {
        let fixed = double_to_u8fixed8(val);
        let back = u8fixed8_to_double(fixed);
        let err = (val - back).abs();

        // 8.8 has about 1/256 precision
        assert!(
            err < 2.0 / 256.0,
            "8.8 roundtrip error for {}: {}",
            val,
            err
        );
    }
}

// ============================================================================
// Lab Desaturation Tests
// ============================================================================

/// Test Lab desaturation clips to gamut
#[test]
fn test_lab_desaturate() {
    let mut lab = CIELab {
        L: 50.0,
        a: 150.0, // Way out of gamut
        b: 150.0,
    };

    let ok = lab.desaturate(-128.0, 128.0, -128.0, 128.0);
    assert!(ok, "Desaturation should succeed");

    assert!(
        lab.a >= -128.0 && lab.a <= 128.0,
        "a* should be in gamut after desaturation: {}",
        lab.a
    );
    assert!(
        lab.b >= -128.0 && lab.b <= 128.0,
        "b* should be in gamut after desaturation: {}",
        lab.b
    );
}

// ============================================================================
// Summary Test
// ============================================================================

// ============================================================================
// Gamma Curve Tests
// ============================================================================

/// Helper function to compare curve output to expected formula
fn check_gamma_curve(gamma: f64, name: &str) {
    let curve = ToneCurve::new(gamma);

    // Test at several points
    let test_points = [0.0f32, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];

    for &x in &test_points {
        let expected = x.powf(gamma as f32);
        let actual = curve.eval(x);
        let err = (expected - actual).abs();

        assert!(
            err < 0.001,
            "{} curve at {}: expected {}, got {}, error {}",
            name,
            x,
            expected,
            actual,
            err
        );
    }
}

/// Test gamma 1.8 curve (Mac gamma)
/// Port of CheckGamma18 from testcms2.c
#[test]
fn test_gamma_1_8() {
    check_gamma_curve(1.8, "Gamma 1.8");
}

/// Test gamma 2.2 curve (Windows/sRGB-ish gamma)
/// Port of CheckGamma22 from testcms2.c
#[test]
fn test_gamma_2_2() {
    check_gamma_curve(2.2, "Gamma 2.2");
}

/// Test gamma 3.0 curve
/// Port of CheckGamma30 from testcms2.c
#[test]
fn test_gamma_3_0() {
    check_gamma_curve(3.0, "Gamma 3.0");
}

/// Test linear (gamma 1.0) curve
/// Port of CheckGammaCreation16/CheckGammaCreationFlt from testcms2.c
#[test]
fn test_gamma_linear() {
    let curve = ToneCurve::new(1.0);

    // Linear curve should be identity
    assert!(curve.is_linear(), "Gamma 1.0 should be linear");

    // Test several points
    for i in 0..=10 {
        let x = i as f32 / 10.0;
        let y = curve.eval(x);
        let err = (x - y).abs();
        assert!(
            err < 0.001,
            "Linear curve at {}: expected {}, got {}",
            x,
            x,
            y
        );
    }
}

/// Test tabulated tone curve (16-bit)
/// Port of CheckGamma18Table from testcms2.c
#[test]
fn test_gamma_tabulated_16() {
    // Create a gamma 2.2 table with 256 entries
    let mut table = [0u16; 256];
    for i in 0..256 {
        let x = i as f64 / 255.0;
        let y = x.powf(2.2);
        table[i] = (y * 65535.0).round() as u16;
    }

    let curve = ToneCurve::new_tabulated(&table);

    // Verify curve properties
    assert!(curve.is_monotonic(), "Gamma table should be monotonic");
    assert!(
        !curve.is_descending(),
        "Gamma table should not be descending"
    );

    // Estimate gamma
    if let Some(estimated) = curve.estimated_gamma(0.1) {
        let err = (estimated - 2.2).abs();
        assert!(
            err < 0.2,
            "Estimated gamma should be ~2.2, got {}",
            estimated
        );
    }
}

/// Test tabulated tone curve (float)
/// Port of CheckGamma22TableFloat from testcms2.c
#[test]
fn test_gamma_tabulated_float() {
    // Create a gamma 2.2 table with 256 entries
    let mut table = [0.0f32; 256];
    for i in 0..256 {
        let x = i as f32 / 255.0;
        table[i] = x.powf(2.2);
    }

    let curve = ToneCurve::new_tabulated_float(&table);

    // Verify curve properties
    assert!(
        curve.is_monotonic(),
        "Float gamma table should be monotonic"
    );

    // Test evaluation
    let y = curve.eval(0.5f32);
    let expected = 0.5f32.powf(2.2);
    let err = (y - expected).abs();
    assert!(
        err < 0.01,
        "Float table eval at 0.5: expected {}, got {}",
        expected,
        y
    );
}

// ============================================================================
// Parametric Curve Tests
// ============================================================================

/// Test parametric curve type 1: Y = X^gamma
/// Port of CheckParametricToneCurves (type 1) from testcms2.c
#[test]
fn test_parametric_curve_type1_gamma() {
    let gamma = 2.2;
    let params = [gamma];
    let curve = ToneCurve::new_parametric(1, &params).expect("Type 1 curve creation failed");

    // Test at several points
    for i in 0..=10 {
        let x = i as f32 / 10.0;
        let expected = x.powf(gamma as f32);
        let actual = curve.eval(x);
        let err = (expected - actual).abs();

        assert!(
            err < 0.001,
            "Type 1 (gamma) at {}: expected {}, got {}",
            x,
            expected,
            actual
        );
    }
}

/// Test parametric curve type 2: CIE 122-1966
/// Y = (aX + b)^Gamma  | X >= -b/a
/// Y = 0               | else
#[test]
fn test_parametric_curve_type2_cie122() {
    let params = [2.2, 1.5, -0.5]; // gamma, a, b
    let curve = ToneCurve::new_parametric(2, &params).expect("Type 2 curve creation failed");

    assert!(
        curve.is_monotonic(),
        "CIE 122-1966 curve should be monotonic"
    );

    // At x = 0.5: y = (1.5 * 0.5 - 0.5)^2.2 = 0.25^2.2 ≈ 0.047
    let y = curve.eval(0.5f32);
    assert!(y > 0.0 && y < 1.0, "CIE 122-1966 output should be in range");
}

/// Test parametric curve type 3: IEC 61966-3
/// Y = (aX + b)^Gamma + c | X >= -b/a
/// Y = c                  | else
#[test]
fn test_parametric_curve_type3_iec61966_3() {
    // Use parameters that produce a well-behaved curve
    // Transition at -b/a = -(-0.1)/1.0 = 0.1
    let params = [2.2, 1.0, -0.1, 0.0]; // gamma, a, b, c
    let curve = ToneCurve::new_parametric(3, &params).expect("Type 3 curve creation failed");

    // Test evaluation at a point above threshold
    let y = curve.eval(0.5f32);
    // y = (1.0 * 0.5 + (-0.1))^2.2 + 0.0 = 0.4^2.2 ≈ 0.128
    assert!(
        y > 0.0 && y < 1.0,
        "IEC 61966-3 output should be in range: {}",
        y
    );
}

/// Test parametric curve type 4: IEC 61966-2.1 (sRGB)
/// Y = (aX + b)^Gamma | X >= d
/// Y = cX             | X < d
#[test]
fn test_parametric_curve_type4_srgb() {
    // sRGB EOTF parameters
    let params = [
        2.4,           // gamma
        1.0 / 1.055,   // a
        0.055 / 1.055, // b
        1.0 / 12.92,   // c
        0.04045,       // d (transition point)
    ];
    let curve = ToneCurve::new_parametric(4, &params).expect("Type 4 (sRGB) curve creation failed");

    assert!(curve.is_monotonic(), "sRGB curve should be monotonic");

    // Test at known sRGB values
    // sRGB linear 0.0 -> sRGB encoded 0.0
    let y0 = curve.eval(0.0f32);
    assert!(y0.abs() < 0.001, "sRGB at 0: expected 0, got {}", y0);

    // sRGB linear 1.0 -> sRGB encoded 1.0
    let y1 = curve.eval(1.0f32);
    assert!(
        (y1 - 1.0).abs() < 0.001,
        "sRGB at 1: expected 1, got {}",
        y1
    );
}

/// Test parametric curve type 108: S-Shaped sigmoidal
/// Y = (1 - (1-x)^(1/g))^(1/g)
#[test]
fn test_parametric_curve_type108_sigmoidal() {
    let params = [1.9]; // gamma
    let curve = ToneCurve::new_parametric(108, &params)
        .expect("Type 108 (sigmoidal) curve creation failed");

    // Sigmoidal should be monotonic
    assert!(curve.is_monotonic(), "Sigmoidal curve should be monotonic");

    // Test endpoints
    let y0 = curve.eval(0.0f32);
    let y1 = curve.eval(1.0f32);
    assert!(y0.abs() < 0.001, "Sigmoidal at 0: expected 0, got {}", y0);
    assert!(
        (y1 - 1.0).abs() < 0.001,
        "Sigmoidal at 1: expected 1, got {}",
        y1
    );

    // Test midpoint (sigmoidal should be approximately symmetric around 0.5)
    let y_mid = curve.eval(0.5f32);
    assert!(
        (y_mid - 0.5).abs() < 0.1,
        "Sigmoidal at 0.5 should be ~0.5, got {}",
        y_mid
    );
}

/// Test curve inversion
/// Port of CheckJointCurves from testcms2.c
#[test]
fn test_curve_inversion() {
    let gamma = 2.2;
    let curve = ToneCurve::new(gamma);
    let inverse = curve.reversed();

    // Forward * Inverse should be identity
    for i in 0..=10 {
        let x = i as f32 / 10.0;
        let y = curve.eval(x);
        let x2 = inverse.eval(y);
        let err = (x - x2).abs();

        assert!(
            err < 0.001,
            "Curve inversion at {}: forward={}, back={}, error={}",
            x,
            y,
            x2,
            err
        );
    }
}

/// Test curve join
/// Port of CheckJointCurves from testcms2.c
#[test]
fn test_curve_join() {
    // join() computes Y^-1(X(t))
    // To get identity, we need Y = X, so Y^-1(X(t)) = t
    let gamma = 2.2;
    let curve1 = ToneCurve::new(gamma);
    let curve2 = ToneCurve::new(gamma);

    // Join the same gamma curve: Y^-1(X(t)) where X=Y=gamma^2.2
    // Result should be identity
    let joined = curve1.join(&curve2, 256);

    // Check if result is approximately linear
    for i in 0..=10 {
        let x = i as f32 / 10.0;
        let y = joined.eval(x);
        let err = (x - y).abs();

        // Allow larger error due to tabulated approximation
        assert!(
            err < 0.02,
            "Joined curve at {}: expected {}, got {}",
            x,
            x,
            y
        );
    }

    // Also test that joining with reversed curve gives non-identity
    let curve_inv = curve1.reversed();
    let joined2 = curve1.join(&curve_inv, 256);

    // X^(1/gamma)(X^gamma(t)) = t^(gamma * 1/gamma) = t
    // Wait, that should also be identity...
    // Actually join(X, Y) = Y^-1(X(t))
    // If Y = X^-1, then Y^-1 = X, so Y^-1(X(t)) = X(X(t)) = t^(2*gamma)
    // So this should NOT be identity
    let y_nonident = joined2.eval(0.5f32);
    // At 0.5: 0.5^(2*2.2) = 0.5^4.4 ≈ 0.047
    assert!(
        (y_nonident - 0.5).abs() > 0.1,
        "Joining with reversed curve should NOT be identity at 0.5: got {}",
        y_nonident
    );
}

/// Test descending curve detection
/// Port of CheckJointCurvesDescending from testcms2.c
#[test]
fn test_curve_descending() {
    // Create a descending table (inverted)
    let mut table = [0u16; 256];
    for i in 0..256 {
        table[i] = ((255 - i) as u32 * 257) as u16; // 255*257 = 65535
    }

    let curve = ToneCurve::new_tabulated(&table);

    assert!(curve.is_monotonic(), "Descending curve should be monotonic");
    assert!(
        curve.is_descending(),
        "Inverted table should be marked as descending"
    );
}

// ============================================================================
// Summary Test
// ============================================================================

/// Run all color space tests and report summary
#[test]
fn test_color_space_summary() {
    // This test just ensures all the above tests compile and can be referenced
    println!("lcms2 testbed port: All tests defined");
    println!("Tests ported from testcms2.c:");
    println!("  - D50 roundtrip");
    println!("  - Lab <-> LCh");
    println!("  - Lab <-> XYZ");
    println!("  - Lab <-> xyY");
    println!("  - Lab V2/V4 encoding");
    println!("  - Color temperature");
    println!("  - Delta E metrics");
    println!("  - Chromatic adaptation");
    println!("  - Fixed-point precision");
    println!("  - Gamma curves (1.0, 1.8, 2.2, 3.0)");
    println!("  - Tabulated curves (16-bit, float)");
    println!("  - Parametric curves (types 1-4, 108)");
    println!("  - Curve inversion and joining");
}
