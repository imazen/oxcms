//! Proof test for moxcms matrix-based profile handling
//!
//! This test proves that rgb_to_xyz_matrix() should use the colorant matrix directly
//! (or equivalently, use colorant sum as white point), NOT hardcoded D50.
//!
//! ## Mathematical Proof
//!
//! The `rgb_to_xyz_d(M, wp)` function computes:
//!   s = M⁻¹ × wp
//!   result = M × diag(s)
//!
//! When wp = colorant_sum = M × [1,1,1]:
//!   s = M⁻¹ × (M × [1,1,1]) = [1,1,1]
//!   result = M × diag([1,1,1]) = M
//!
//! Therefore: rgb_to_xyz_d(M, colorant_sum) ≡ M
//!
//! This matches what skcms and lcms2 do: use the colorant matrix directly.
//!
//! ## ICC Specification References (ICC.1:2022-05)
//!
//! ### Section F.3 - Three-component matrix-based profiles (pp. 125-126)
//!
//! > "This model describes transformation from device colour space to PCS.
//! > The transformation is based on three non-interdependent per-channel tone
//! > reproduction curves to convert between non-linear and linear RGB values
//! > and a 3×3 matrix to convert between linear RGB values and relative XYZ values."
//!
//! The computational model (Equation F.6):
//! ```text
//! ┌ connectionX ┐   ┌ redMatrixColumnX   greenMatrixColumnX   blueMatrixColumnX ┐ ┌ linearR ┐
//! │ connectionY │ = │ redMatrixColumnY   greenMatrixColumnY   blueMatrixColumnY │ │ linearG │
//! └ connectionZ ┘   └ redMatrixColumnZ   greenMatrixColumnZ   blueMatrixColumnZ ┘ └ linearB ┘
//! ```
//!
//! **Key observation**: The spec uses colorant tags DIRECTLY as matrix columns.
//! No white point scaling is mentioned in the computational model.
//!
//! ### Section 8.2 - Common requirements (p. 40)
//!
//! > "chromaticAdaptationTag, when the measurement data used to calculate the
//! > profile was specified for an adopted white with a chromaticity different
//! > from that of the PCS adopted white"
//!
//! This means CHAD tag is only required when adaptation is needed.
//!
//! ### Annex E.4.1 - Adjustments using chromatic adaptation tag (p. 123)
//!
//! > "Only one profile has the chromaticAdaptationTag. Processing is
//! > **implementation dependent**."
//!
//! This explicitly states that handling profiles without CHAD tag is implementation-defined.
//!
//! ## Conclusion
//!
//! The ICC spec F.3 defines the matrix model as direct colorant matrix multiplication.
//! Using hardcoded D50 white point to scale the matrix is NOT specified by the ICC.
//! Both skcms and lcms2 implement F.3 correctly by using colorants directly.

use std::path::Path;

/// Test profile data - SM245B.icc has D65 colorants without CHAD tag
fn get_sm245b_path() -> Option<std::path::PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("testdata/profiles/skcms/misc/SM245B.icc");
    if path.exists() { Some(path) } else { None }
}

/// Prove that colorant_sum as white point produces identity scaling
#[test]
fn test_colorant_sum_produces_identity_scaling() {
    // Create a test matrix (arbitrary invertible matrix)
    let m = moxcms::Matrix3d {
        v: [
            [0.4, 0.3, 0.2],
            [0.2, 0.7, 0.1],
            [0.0, 0.1, 0.9],
        ],
    };

    // Colorant sum = M × [1,1,1]
    let colorant_sum = moxcms::Xyzd {
        x: m.v[0][0] + m.v[0][1] + m.v[0][2],
        y: m.v[1][0] + m.v[1][1] + m.v[1][2],
        z: m.v[2][0] + m.v[2][1] + m.v[2][2],
    };

    // rgb_to_xyz_d with colorant_sum should return M unchanged
    let result = moxcms::ColorProfile::rgb_to_xyz_d(m, colorant_sum);

    // Verify result ≈ M (within floating point tolerance)
    for i in 0..3 {
        for j in 0..3 {
            let diff = (result.v[i][j] - m.v[i][j]).abs();
            assert!(
                diff < 1e-10,
                "rgb_to_xyz_d(M, colorant_sum) should equal M, but [{i}][{j}] differs by {diff}"
            );
        }
    }

    eprintln!("✓ PROVED: rgb_to_xyz_d(M, colorant_sum) = M (identity scaling)");
}

/// Prove that D50 produces NON-identity scaling for non-D50 colorants
#[test]
fn test_d50_produces_wrong_scaling_for_d65_colorants() {
    // SM245B-like colorants (sum to D65, not D50)
    let m = moxcms::Matrix3d {
        v: [
            [0.458725, 0.322952, 0.168488],  // Sum ≈ 0.950
            [0.232895, 0.697388, 0.069717],  // Sum = 1.000
            [0.014114, 0.149780, 0.923767],  // Sum ≈ 1.088
        ],
    };

    let d50 = moxcms::Chromaticity::D50.to_xyzd();

    // rgb_to_xyz_d with D50 should NOT return M unchanged
    let result = moxcms::ColorProfile::rgb_to_xyz_d(m, d50);

    // Calculate max difference
    let mut max_diff: f64 = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            let diff = (result.v[i][j] - m.v[i][j]).abs();
            max_diff = max_diff.max(diff);
        }
    }

    // The difference should be significant (proves D50 is wrong for this matrix)
    assert!(
        max_diff > 0.01,
        "D50 scaling should produce significantly different matrix for D65 colorants, but max_diff = {max_diff}"
    );

    eprintln!("✓ PROVED: rgb_to_xyz_d(M, D50) ≠ M for D65 colorants (max_diff = {max_diff:.6})");
    eprintln!("  This proves hardcoded D50 is WRONG for profiles with non-D50 colorants");
}

/// Prove moxcms matches skcms for SM245B profile
#[test]
fn test_moxcms_matches_skcms_for_sm245b() {
    let Some(profile_path) = get_sm245b_path() else {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    };

    let profile_data = std::fs::read(&profile_path).unwrap();

    // Parse with both CMS implementations
    let mox_profile = moxcms::ColorProfile::new_from_slice(&profile_data).unwrap();
    let skcms_profile = skcms_sys::parse_icc_profile(&profile_data).unwrap();

    // Get matrices
    let mox_matrix = mox_profile.rgb_to_xyz_matrix();

    eprintln!("\nmoxcms rgb_to_xyz_matrix:");
    for i in 0..3 {
        eprintln!("  [{:.6}, {:.6}, {:.6}]", mox_matrix.v[i][0], mox_matrix.v[i][1], mox_matrix.v[i][2]);
    }

    eprintln!("\nskcms toXYZD50:");
    for i in 0..3 {
        eprintln!(
            "  [{:.6}, {:.6}, {:.6}]",
            skcms_profile.toXYZD50.vals[i][0],
            skcms_profile.toXYZD50.vals[i][1],
            skcms_profile.toXYZD50.vals[i][2]
        );
    }

    // Compare matrices
    let mut max_diff: f32 = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            let mox_val = mox_matrix.v[i][j] as f32;
            let skcms_val = skcms_profile.toXYZD50.vals[i][j];
            let diff = (mox_val - skcms_val).abs();
            max_diff = max_diff.max(diff);
        }
    }

    eprintln!("\nMax matrix difference: {:.9}", max_diff);

    assert!(
        max_diff < 0.0001,
        "moxcms matrix should match skcms, but max_diff = {max_diff}"
    );

    eprintln!("✓ PROVED: moxcms rgb_to_xyz_matrix matches skcms toXYZD50");
}

/// Prove transform outputs match between all three CMS implementations
#[test]
fn test_transform_outputs_match_all_cms() {
    let Some(profile_path) = get_sm245b_path() else {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    };

    let profile_data = std::fs::read(&profile_path).unwrap();

    let test_colors: &[([u8; 3], &str)] = &[
        ([255, 255, 255], "White"),
        ([128, 128, 128], "Gray"),
        ([255, 0, 0], "Red"),
        ([0, 255, 0], "Green"),
        ([0, 0, 255], "Blue"),
        ([255, 128, 0], "Orange"),
        ([128, 0, 255], "Purple"),
    ];

    eprintln!("\nTransform comparison (SM245B → sRGB):");
    eprintln!("{:<12} {:>15} {:>15} {:>15}", "Color", "lcms2", "skcms", "moxcms");
    eprintln!("{}", "-".repeat(60));

    let mut total_lcms_skcms_diff = 0i32;
    let mut total_mox_skcms_diff = 0i32;

    for (rgb, name) in test_colors {
        // lcms2
        let lcms_out = {
            let profile = lcms2::Profile::new_icc(&profile_data).unwrap();
            let srgb = lcms2::Profile::new_srgb();
            let transform = lcms2::Transform::<[u8; 3], [u8; 3]>::new(
                &profile, lcms2::PixelFormat::RGB_8,
                &srgb, lcms2::PixelFormat::RGB_8,
                lcms2::Intent::Perceptual,
            ).unwrap();
            let mut out = [0u8; 3];
            transform.transform_pixels(std::slice::from_ref(rgb), std::slice::from_mut(&mut out));
            out
        };

        // skcms
        let skcms_out = {
            let profile = skcms_sys::parse_icc_profile(&profile_data).unwrap();
            let srgb = skcms_sys::srgb_profile();
            let mut out = [0u8; 3];
            skcms_sys::transform(
                rgb,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &profile,
                &mut out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                srgb,
                1,
            );
            out
        };

        // moxcms
        let mox_out = {
            let profile = moxcms::ColorProfile::new_from_slice(&profile_data).unwrap();
            let srgb = moxcms::ColorProfile::new_srgb();
            let transform = profile.create_transform_8bit(
                moxcms::Layout::Rgb, &srgb, moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            ).unwrap();
            let mut out = [0u8; 3];
            transform.transform(rgb, &mut out).unwrap();
            out
        };

        eprintln!(
            "{:<12} [{:3},{:3},{:3}] [{:3},{:3},{:3}] [{:3},{:3},{:3}]",
            name,
            lcms_out[0], lcms_out[1], lcms_out[2],
            skcms_out[0], skcms_out[1], skcms_out[2],
            mox_out[0], mox_out[1], mox_out[2],
        );

        // Track differences
        for i in 0..3 {
            total_lcms_skcms_diff += (lcms_out[i] as i32 - skcms_out[i] as i32).abs();
            total_mox_skcms_diff += (mox_out[i] as i32 - skcms_out[i] as i32).abs();
        }
    }

    eprintln!("\nTotal difference lcms2 vs skcms: {}", total_lcms_skcms_diff);
    eprintln!("Total difference moxcms vs skcms: {}", total_mox_skcms_diff);

    assert_eq!(total_mox_skcms_diff, 0, "moxcms should exactly match skcms");
    assert!(total_lcms_skcms_diff <= 7, "lcms2 should closely match skcms");

    eprintln!("\n✓ PROVED: All three CMS implementations produce matching outputs");
}

/// Prove the mathematical identity: rgb_to_xyz_d(M, M×[1,1,1]) = M
#[test]
fn test_mathematical_identity_formal_proof() {
    eprintln!("\n=== FORMAL MATHEMATICAL PROOF ===\n");

    // Use exact rational-like values to minimize floating point issues
    let m = moxcms::Matrix3d {
        v: [
            [0.5, 0.25, 0.125],
            [0.25, 0.5, 0.25],
            [0.125, 0.25, 0.5],
        ],
    };

    eprintln!("Given matrix M:");
    for i in 0..3 {
        eprintln!("  [{:8.6}, {:8.6}, {:8.6}]", m.v[i][0], m.v[i][1], m.v[i][2]);
    }

    // Step 1: Compute colorant_sum = M × [1,1,1]
    let colorant_sum = moxcms::Xyzd {
        x: m.v[0][0] + m.v[0][1] + m.v[0][2],
        y: m.v[1][0] + m.v[1][1] + m.v[1][2],
        z: m.v[2][0] + m.v[2][1] + m.v[2][2],
    };
    eprintln!("\nStep 1: colorant_sum = M × [1,1,1] = [{:.6}, {:.6}, {:.6}]",
              colorant_sum.x, colorant_sum.y, colorant_sum.z);

    // Step 2: Compute M⁻¹
    let m_inv = m.inverse();
    eprintln!("\nStep 2: M⁻¹ =");
    for i in 0..3 {
        eprintln!("  [{:8.6}, {:8.6}, {:8.6}]", m_inv.v[i][0], m_inv.v[i][1], m_inv.v[i][2]);
    }

    // Step 3: Compute s = M⁻¹ × colorant_sum
    let s = m_inv.mul_vector(colorant_sum.to_vector_d());
    eprintln!("\nStep 3: s = M⁻¹ × colorant_sum = [{:.6}, {:.6}, {:.6}]", s.v[0], s.v[1], s.v[2]);

    // Verify s ≈ [1,1,1]
    assert!((s.v[0] - 1.0).abs() < 1e-10, "s[0] should be 1.0");
    assert!((s.v[1] - 1.0).abs() < 1e-10, "s[1] should be 1.0");
    assert!((s.v[2] - 1.0).abs() < 1e-10, "s[2] should be 1.0");
    eprintln!("  ✓ s = [1, 1, 1] (as expected)");

    // Step 4: Compute result = M × diag(s) = M × I = M
    let result = moxcms::ColorProfile::rgb_to_xyz_d(m, colorant_sum);
    eprintln!("\nStep 4: result = M × diag(s) =");
    for i in 0..3 {
        eprintln!("  [{:8.6}, {:8.6}, {:8.6}]", result.v[i][0], result.v[i][1], result.v[i][2]);
    }

    // Verify result = M
    for i in 0..3 {
        for j in 0..3 {
            let diff = (result.v[i][j] - m.v[i][j]).abs();
            assert!(diff < 1e-10, "result[{i}][{j}] should equal M[{i}][{j}]");
        }
    }

    eprintln!("\n=== QED: rgb_to_xyz_d(M, M×[1,1,1]) = M ===");
    eprintln!("\nThis proves that using colorant_sum as white_point is equivalent to");
    eprintln!("returning the colorant matrix directly, which is what skcms and lcms2 do.");
}
