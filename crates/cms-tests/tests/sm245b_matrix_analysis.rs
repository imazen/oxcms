//! SM245B.icc Matrix-Based Profile Analysis
//!
//! This test provides rigorous analysis of how different CMS implementations
//! handle the Samsung SM245B monitor profile, which has colorants that sum
//! to D65 rather than D50.
//!
//! ## ICC Specification References
//!
//! ### ICC.1:2022-05 Section F.3 (Three-component matrix-based profiles)
//!
//! The spec defines the computational model as (Equation F.6):
//!
//! ```text
//! [connectionX]   [redMatrixColumnX   greenMatrixColumnX   blueMatrixColumnX] [linearR]
//! [connectionY] = [redMatrixColumnY   greenMatrixColumnY   blueMatrixColumnY] [linearG]
//! [connectionZ]   [redMatrixColumnZ   greenMatrixColumnZ   blueMatrixColumnZ] [linearB]
//! ```
//!
//! **Key observation**: The spec uses the colorant tags DIRECTLY as matrix columns.
//! There is NO white point scaling mentioned in the computational model.
//!
//! ### ICC.1:2022-05 Section 8.2 (Common requirements)
//!
//! > chromaticAdaptationTag, when the measurement data used to calculate the
//! > profile was specified for an adopted white with a chromaticity different
//! > from that of the PCS adopted white (see 9.2.15).
//!
//! ### ICC.1:2022-05 Annex E.4.1 (Adjustments using chromatic adaptation tag)
//!
//! > Only one profile has the chromaticAdaptationTag. Processing is
//! > **implementation dependent**.
//!
//! ## SM245B Profile Analysis
//!
//! - **Version**: 2.0.2 (v2 profile)
//! - **Colorant sum**: [0.950165, 1.000000, 1.087662] ≈ D65
//! - **D50 reference**: [0.964212, 1.000000, 0.825188]
//! - **CHAD tag**: Not present
//!
//! The profile has unadapted colorants (D65) without a chromaticAdaptationTag.
//! Per ICC spec Annex E.4.1, handling is "implementation dependent."

use std::path::{Path, PathBuf};
use std::slice;

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Extract the 3x3 colorant matrix from a profile
fn extract_colorant_matrix(profile: &moxcms::ColorProfile) -> [[f64; 3]; 3] {
    [
        [
            profile.red_colorant.x,
            profile.green_colorant.x,
            profile.blue_colorant.x,
        ],
        [
            profile.red_colorant.y,
            profile.green_colorant.y,
            profile.blue_colorant.y,
        ],
        [
            profile.red_colorant.z,
            profile.green_colorant.z,
            profile.blue_colorant.z,
        ],
    ]
}

/// Transform using lcms2
fn transform_lcms2(profile_data: &[u8], rgb: [u8; 3]) -> Option<[u8; 3]> {
    let profile = lcms2::Profile::new_icc(profile_data).ok()?;
    let srgb = lcms2::Profile::new_srgb();

    let transform = lcms2::Transform::<[u8; 3], [u8; 3]>::new(
        &profile,
        lcms2::PixelFormat::RGB_8,
        &srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .ok()?;

    let mut out = [0u8; 3];
    transform.transform_pixels(slice::from_ref(&rgb), slice::from_mut(&mut out));
    Some(out)
}

/// Transform using skcms
fn transform_skcms(profile_data: &[u8], rgb: [u8; 3]) -> Option<[u8; 3]> {
    let profile = skcms_sys::parse_icc_profile(profile_data)?;
    let srgb = skcms_sys::srgb_profile();

    let mut out = [0u8; 3];
    let success = skcms_sys::transform(
        &rgb,
        skcms_sys::skcms_PixelFormat::RGB_888,
        skcms_sys::skcms_AlphaFormat::Opaque,
        &profile,
        &mut out,
        skcms_sys::skcms_PixelFormat::RGB_888,
        skcms_sys::skcms_AlphaFormat::Opaque,
        srgb,
        1,
    );

    if success { Some(out) } else { None }
}

/// Transform using moxcms
fn transform_moxcms(profile_data: &[u8], rgb: [u8; 3]) -> Option<[u8; 3]> {
    let profile = moxcms::ColorProfile::new_from_slice(profile_data).ok()?;
    let srgb = moxcms::ColorProfile::new_srgb();

    let transform = profile
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .ok()?;

    let mut out = [0u8; 3];
    transform.transform(&rgb, &mut out).ok()?;
    Some(out)
}

/// Format matrix for display
fn format_matrix(m: &[[f64; 3]; 3]) -> String {
    format!(
        "  [{:.6}, {:.6}, {:.6}]\n  [{:.6}, {:.6}, {:.6}]\n  [{:.6}, {:.6}, {:.6}]",
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    )
}

#[test]
fn test_sm245b_comprehensive_analysis() {
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("SM245B.icc COMPREHENSIVE MATRIX ANALYSIS");
    eprintln!("{}\n", "=".repeat(80));

    let profile_path = testdata_dir().join("profiles/skcms/misc/SM245B.icc");
    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found at {:?}", profile_path);
        return;
    }

    let profile_data = std::fs::read(&profile_path).unwrap();
    eprintln!("Profile: SM245B.icc ({} bytes)", profile_data.len());

    // Parse with moxcms to extract colorants
    let mox_profile = moxcms::ColorProfile::new_from_slice(&profile_data).unwrap();

    // === SECTION 1: Profile Metadata ===
    eprintln!("\n## 1. Profile Metadata\n");
    eprintln!("Version: {:?}", mox_profile.version());
    eprintln!(
        "Has CHAD tag: {}",
        mox_profile.chromatic_adaptation.is_some()
    );

    // === SECTION 2: Colorant Analysis ===
    eprintln!("\n## 2. Colorant Analysis\n");

    let colorant_matrix = extract_colorant_matrix(&mox_profile);
    eprintln!("Colorant Matrix (from rXYZ, gXYZ, bXYZ tags):");
    eprintln!("{}", format_matrix(&colorant_matrix));

    let sum_x = colorant_matrix[0][0] + colorant_matrix[0][1] + colorant_matrix[0][2];
    let sum_y = colorant_matrix[1][0] + colorant_matrix[1][1] + colorant_matrix[1][2];
    let sum_z = colorant_matrix[2][0] + colorant_matrix[2][1] + colorant_matrix[2][2];
    eprintln!(
        "\nColorant Sum (R+G+B): [{:.6}, {:.6}, {:.6}]",
        sum_x, sum_y, sum_z
    );

    // Reference white points
    let d50 = moxcms::Chromaticity::D50.to_xyzd();
    let d65 = moxcms::Chromaticity::D65.to_xyzd();
    eprintln!(
        "D50 Reference:        [{:.6}, {:.6}, {:.6}]",
        d50.x, d50.y, d50.z
    );
    eprintln!(
        "D65 Reference:        [{:.6}, {:.6}, {:.6}]",
        d65.x, d65.y, d65.z
    );

    // Calculate distances
    let dist_d50 =
        ((sum_x - d50.x).powi(2) + (sum_y - d50.y).powi(2) + (sum_z - d50.z).powi(2)).sqrt();
    let dist_d65 =
        ((sum_x - d65.x).powi(2) + (sum_y - d65.y).powi(2) + (sum_z - d65.z).powi(2)).sqrt();
    eprintln!("\nDistance from D50: {:.6}", dist_d50);
    eprintln!("Distance from D65: {:.6}", dist_d65);
    eprintln!(
        "Colorant sum is closer to: {}",
        if dist_d50 < dist_d65 { "D50" } else { "D65" }
    );

    // === SECTION 3: ICC Spec Reference ===
    eprintln!("\n## 3. ICC Specification Reference\n");
    eprintln!("ICC.1:2022-05 Section F.3 defines the matrix model as:");
    eprintln!("  connection = colorantMatrix × linear_rgb");
    eprintln!("  (No white point scaling is specified in the computational model)");
    eprintln!("");
    eprintln!("ICC.1:2022-05 Annex E.4.1 states:");
    eprintln!("  'Only one profile has chromaticAdaptationTag.");
    eprintln!("   Processing is implementation dependent.'");
    eprintln!("");
    eprintln!("SM245B has NO chromaticAdaptationTag, so handling is implementation-defined.");

    // === SECTION 4: CMS Implementation Analysis ===
    eprintln!("\n## 4. CMS Implementation Behavior\n");

    // skcms analysis
    eprintln!("### skcms (skcms.cc lines 562-566):");
    eprintln!("  read_to_XYZD50() reads rXYZ/gXYZ/bXYZ tags DIRECTLY into toXYZD50 matrix.");
    eprintln!("  No white point scaling or Bradford adaptation is applied.");
    eprintln!("  The colorant matrix is used as-is.");

    // lcms2 analysis
    eprintln!("\n### lcms2 (cmsio1.c lines 139-148, 219-227):");
    eprintln!("  ReadICCMatrixRGB2XYZ() reads colorant tags directly into Mat.");
    eprintln!("  Only encoding format adjustment (InpAdj) is applied.");
    eprintln!("  No white point scaling is applied to the colorant matrix.");

    // moxcms analysis
    eprintln!("\n### moxcms:");
    eprintln!("  With fix: Uses colorant sum as white point in rgb_to_xyz_matrix().");
    eprintln!("  Original: Used D50 as white point unconditionally.");

    // === SECTION 5: Transform Comparison ===
    eprintln!("\n## 5. Transform Comparison\n");

    let test_colors: &[([u8; 3], &str)] = &[
        ([255, 255, 255], "White"),
        ([128, 128, 128], "Gray 50%"),
        ([0, 0, 0], "Black"),
        ([255, 0, 0], "Red"),
        ([0, 255, 0], "Green"),
        ([0, 0, 255], "Blue"),
    ];

    eprintln!(
        "{:<15} {:>20} {:>20} {:>20}",
        "Input", "lcms2", "skcms", "moxcms"
    );
    eprintln!("{}", "-".repeat(80));

    let mut max_lcms_skcms_diff = 0i32;
    let mut max_mox_skcms_diff = 0i32;

    for (rgb, name) in test_colors {
        let lcms_out = transform_lcms2(&profile_data, *rgb);
        let skcms_out = transform_skcms(&profile_data, *rgb);
        let mox_out = transform_moxcms(&profile_data, *rgb);

        let format_rgb = |o: Option<[u8; 3]>| match o {
            Some([r, g, b]) => format!("[{:3},{:3},{:3}]", r, g, b),
            None => "  FAILED  ".to_string(),
        };

        eprintln!(
            "{:<15} {:>20} {:>20} {:>20}",
            name,
            format_rgb(lcms_out),
            format_rgb(skcms_out),
            format_rgb(mox_out)
        );

        // Calculate max differences
        if let (Some(l), Some(s)) = (lcms_out, skcms_out) {
            let diff = (0..3)
                .map(|i| (l[i] as i32 - s[i] as i32).abs())
                .max()
                .unwrap();
            max_lcms_skcms_diff = max_lcms_skcms_diff.max(diff);
        }
        if let (Some(m), Some(s)) = (mox_out, skcms_out) {
            let diff = (0..3)
                .map(|i| (m[i] as i32 - s[i] as i32).abs())
                .max()
                .unwrap();
            max_mox_skcms_diff = max_mox_skcms_diff.max(diff);
        }
    }

    eprintln!("\nMax difference lcms2 vs skcms: {}", max_lcms_skcms_diff);
    eprintln!("Max difference moxcms vs skcms: {}", max_mox_skcms_diff);

    // === SECTION 6: Verification ===
    eprintln!("\n## 6. Verification\n");

    if max_mox_skcms_diff <= 2 {
        eprintln!(
            "✓ moxcms matches skcms within tolerance (max diff: {})",
            max_mox_skcms_diff
        );
    } else {
        eprintln!(
            "✗ moxcms differs from skcms significantly (max diff: {})",
            max_mox_skcms_diff
        );
    }

    if max_lcms_skcms_diff <= 2 {
        eprintln!(
            "✓ lcms2 matches skcms within tolerance (max diff: {})",
            max_lcms_skcms_diff
        );
    } else {
        eprintln!(
            "✗ lcms2 differs from skcms significantly (max diff: {})",
            max_lcms_skcms_diff
        );
    }

    // === SECTION 7: Conclusions ===
    eprintln!("\n## 7. Conclusions\n");
    eprintln!("1. SM245B.icc has colorants summing to D65, not D50.");
    eprintln!("2. It lacks a chromaticAdaptationTag, making handling 'implementation defined'.");
    eprintln!("3. Both skcms and lcms2 use the colorant matrix DIRECTLY per ICC F.3.");
    eprintln!("4. Neither applies white point scaling to the colorant matrix.");
    eprintln!("5. For spec compliance, the fix should match skcms/lcms2 behavior.");

    eprintln!("\n{}\n", "=".repeat(80));

    // Assertions for test verification
    assert!(max_mox_skcms_diff <= 2, "moxcms should match skcms");
}

#[test]
fn test_verify_skcms_uses_colorants_directly() {
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("VERIFYING SKCMS USES COLORANT MATRIX DIRECTLY");
    eprintln!("{}\n", "=".repeat(80));

    let profile_path = testdata_dir().join("profiles/skcms/misc/SM245B.icc");
    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let profile_data = std::fs::read(&profile_path).unwrap();
    let mox_profile = moxcms::ColorProfile::new_from_slice(&profile_data).unwrap();

    // Get the colorant matrix from moxcms
    let colorant_matrix = extract_colorant_matrix(&mox_profile);

    // The skcms toXYZD50 matrix should be exactly the colorant matrix
    // We can verify this by checking that transforming [1,0,0] gives the red colorant
    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&profile_data) {
        // skcms stores toXYZD50 as the colorant matrix
        // We can access it through the profile struct
        eprintln!("skcms profile parsed successfully");
        eprintln!("has_toXYZD50: {}", skcms_profile.has_toXYZD50);

        if skcms_profile.has_toXYZD50 {
            eprintln!("\nskcms toXYZD50 matrix:");
            for i in 0..3 {
                eprintln!(
                    "  [{:.6}, {:.6}, {:.6}]",
                    skcms_profile.toXYZD50.vals[i][0],
                    skcms_profile.toXYZD50.vals[i][1],
                    skcms_profile.toXYZD50.vals[i][2]
                );
            }

            eprintln!("\nColorant matrix from ICC tags:");
            eprintln!("{}", format_matrix(&colorant_matrix));

            // Verify they match
            let mut max_diff: f32 = 0.0;
            for i in 0..3 {
                for j in 0..3 {
                    let skcms_val = skcms_profile.toXYZD50.vals[i][j];
                    let icc_val = colorant_matrix[i][j] as f32;
                    let diff = (skcms_val - icc_val).abs();
                    max_diff = max_diff.max(diff);
                }
            }

            eprintln!(
                "\nMax difference between skcms toXYZD50 and ICC colorants: {:.9}",
                max_diff
            );

            if max_diff < 0.0001 {
                eprintln!("✓ VERIFIED: skcms toXYZD50 = colorant matrix (no white point scaling)");
            } else {
                eprintln!("✗ Matrices differ unexpectedly");
            }

            assert!(
                max_diff < 0.0001,
                "skcms should use colorant matrix directly"
            );
        }
    }

    eprintln!("\n{}\n", "=".repeat(80));
}
