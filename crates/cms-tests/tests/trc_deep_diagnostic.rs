//! Deep TRC Diagnostic Test
//!
//! Examines the raw TRC curve data and linearization tables to find the bug.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Parse the SM245B profile and examine its TRC curve
#[test]
fn examine_sm245b_raw_trc() {
    eprintln!("\n=== SM245B Raw TRC Examination ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Parse with moxcms and examine the TRC
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    // Check TRC curve types
    if let Some(ref red_trc) = profile.red_trc {
        match red_trc {
            moxcms::ToneReprCurve::Lut(lut) => {
                eprintln!("Red TRC is LUT with {} entries", lut.len());
                // Print some sample values
                if !lut.is_empty() {
                    eprintln!("  First 5 values: {:?}", &lut[..5.min(lut.len())]);
                    eprintln!("  Last 5 values: {:?}", &lut[lut.len().saturating_sub(5)..]);

                    // Check if this is a gamma curve (monotonically increasing)
                    let is_monotonic = lut.windows(2).all(|w| w[0] <= w[1]);
                    eprintln!("  Monotonically increasing: {}", is_monotonic);

                    // Check the range
                    let min = *lut.iter().min().unwrap();
                    let max = *lut.iter().max().unwrap();
                    eprintln!("  Range: {} to {}", min, max);

                    // Sample at key points (0%, 25%, 50%, 75%, 100%)
                    let len = lut.len();
                    eprintln!("  Sample values:");
                    for &pct in &[0, 25, 50, 75, 100] {
                        let idx = (pct * (len - 1)) / 100;
                        eprintln!(
                            "    {}%: idx={}, value={} ({:.6} normalized)",
                            pct,
                            idx,
                            lut[idx],
                            lut[idx] as f32 / 65535.0
                        );
                    }

                    // Calculate effective gamma at midpoint
                    let mid_idx = lut.len() / 2;
                    let mid_input = 0.5f32;
                    let mid_output = lut[mid_idx] as f32 / 65535.0;
                    eprintln!("\n  Midpoint analysis:");
                    eprintln!("    Input (encoded): {:.3}", mid_input);
                    eprintln!("    Output (linear): {:.4}", mid_output);

                    if mid_output > 0.0 && mid_input > 0.0 && mid_output < 1.0 {
                        let effective_gamma = mid_output.ln() / mid_input.ln();
                        eprintln!("    Effective gamma: {:.3}", effective_gamma);
                    }

                    // For sRGB, 0.5 encoded = ~0.214 linear
                    eprintln!("\n  Reference: sRGB at 0.5 encoded = ~0.214 linear");
                    if mid_output > 0.4 {
                        eprintln!("  => SM245B TRC is FLATTER than sRGB (less gamma)");
                    } else if mid_output < 0.15 {
                        eprintln!("  => SM245B TRC is STEEPER than sRGB (more gamma)");
                    } else {
                        eprintln!("  => SM245B TRC is similar to sRGB");
                    }
                }
            }
            moxcms::ToneReprCurve::Parametric(params) => {
                eprintln!(
                    "Red TRC is Parametric with {} params: {:?}",
                    params.len(),
                    params
                );
            }
        }
    } else {
        eprintln!("No red TRC found!");
    }

    // For comparison, let's see what sRGB's TRC looks like
    let srgb = moxcms::ColorProfile::new_srgb();
    if let Some(ref red_trc) = srgb.red_trc {
        match red_trc {
            moxcms::ToneReprCurve::Lut(lut) => {
                eprintln!("\nsRGB Red TRC is LUT with {} entries", lut.len());
            }
            moxcms::ToneReprCurve::Parametric(params) => {
                eprintln!("\nsRGB Red TRC is Parametric: {:?}", params);
            }
        }
    }
}

/// Test what skcms sees for the same profile
#[test]
fn compare_skcms_vs_moxcms_transforms() {
    eprintln!("\n=== skcms vs moxcms Transform Comparison ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Use skcms
    if let Some(skcms_profile) = skcms_sys::parse_icc_profile(&data) {
        let skcms_srgb = skcms_sys::srgb_profile();

        eprintln!("SM245B -> sRGB comparison:");
        eprintln!("Input | skcms | moxcms | diff");
        eprintln!("------|-------|--------|-----");

        let moxcms_profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = moxcms_profile
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &moxcms_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        for v in [0u8, 32, 64, 96, 128, 160, 192, 224, 255] {
            let color = [v, v, v];

            let mut skcms_out = [0u8; 3];
            skcms_sys::transform(
                &color,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                &skcms_profile,
                &mut skcms_out,
                skcms_sys::skcms_PixelFormat::RGB_888,
                skcms_sys::skcms_AlphaFormat::Opaque,
                skcms_srgb,
                1,
            );

            let mut moxcms_out = [0u8; 3];
            moxcms_transform.transform(&color, &mut moxcms_out).unwrap();

            let diff = moxcms_out[0] as i32 - skcms_out[0] as i32;
            eprintln!(
                " {:3}  |  {:3}  |   {:3}  | {:+3}",
                v, skcms_out[0], moxcms_out[0], diff
            );
        }
    }
}

/// Examine the actual TRC curve shape
#[test]
fn examine_trc_curve_shape() {
    eprintln!("\n=== TRC Curve Shape Analysis ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    if let Some(moxcms::ToneReprCurve::Lut(lut)) = &profile.red_trc {
        eprintln!("SM245B TRC curve shape:");
        eprintln!("Input%  | LUT Index | Raw Value | Normalized");
        eprintln!("--------|-----------|-----------|----------");

        for pct in (0..=100).step_by(10) {
            let idx = (pct * (lut.len() - 1)) / 100;
            let val = lut[idx];
            let normalized = val as f32 / 65535.0;
            eprintln!(
                "  {:3}%  |   {:5}   |   {:5}   |   {:.4}",
                pct, idx, val, normalized
            );
        }

        eprintln!("\n=== Interpretation ===");
        eprintln!("The TRC in ICC profile maps: encoded (input) -> linear (output)");
        eprintln!("For sRGB-like gamma: input 0.5 -> output ~0.214");
        eprintln!("For linear: input 0.5 -> output 0.5");

        let mid_idx = lut.len() / 2;
        let mid_output = lut[mid_idx] as f32 / 65535.0;
        eprintln!("SM245B at 50%: input 0.5 -> output {:.4}", mid_output);

        if mid_output > 0.4 && mid_output < 0.6 {
            eprintln!("=> SM245B appears to be NEARLY LINEAR");
            eprintln!("   This is unusual for a display profile!");
            eprintln!("   Most displays have gamma ~2.2");
        } else if mid_output < 0.3 {
            eprintln!("=> SM245B has gamma similar to sRGB (~2.2-2.4)");
        }
    }
}

/// Compare what linearization does in practice
#[test]
fn compare_linearization_behavior() {
    eprintln!("\n=== Linearization Behavior Comparison ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();

    // Check what the raw TRC data says
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    if let Some(moxcms::ToneReprCurve::Lut(lut)) = &profile.red_trc {
        eprintln!("SM245B TRC LUT has {} entries", lut.len());

        // The TRC LUT maps encoded values (0-65535 corresponding to input indices)
        // to linear values (0-65535 in the LUT values)

        // For a typical gamma 2.2 curve:
        // - Input 32768 (50%) should give output ~14000 (~21.4%)

        // For a linear curve:
        // - Input 32768 (50%) should give output 32768 (50%)

        let mid_idx = lut.len() / 2;
        let mid_val = lut[mid_idx];
        eprintln!(
            "\nAt 50% input (index {}): output = {} ({:.1}%)",
            mid_idx,
            mid_val,
            mid_val as f32 / 655.35
        );

        // Calculate what this means for gamma
        if mid_val > 30000 {
            eprintln!("This suggests a SHALLOW gamma curve (close to linear)");
        } else if mid_val < 20000 {
            eprintln!("This suggests a STEEP gamma curve (like sRGB ~2.2)");
        }

        // Now the KEY QUESTION: Is moxcms applying this correctly?
        // Let's trace through what should happen:
        //
        // For SM245B -> sRGB transform at input 128 (50%):
        // 1. Apply SM245B linearize: 128 -> look up in SM245B's TRC table
        //    - If TRC[128] = 32768 normalized, linear = 0.5
        //    - If TRC[128] = 14000 normalized, linear = 0.214
        //
        // 2. Apply matrix: For gray, matrix preserves the value
        //
        // 3. Apply sRGB gamma: Convert linear back to encoded
        //    - If linear = 0.5, encoded = ~0.735 = 187
        //    - If linear = 0.214, encoded = ~0.5 = 128
        //
        // skcms output: 128 -> 118
        // This means: linearized value is LESS than sRGB would give
        // So SM245B's TRC has STEEPER gamma than sRGB

        // But moxcms output: 128 -> 129
        // This means: moxcms is getting linear ≈ 0.214 and gamma back ≈ 128
        // That would only happen if SM245B and sRGB have the SAME TRC!

        eprintln!("\n=== Expected vs Actual ===");
        eprintln!("skcms: 128 -> 118 (SM245B TRC applied, then sRGB gamma)");
        eprintln!("moxcms: 128 -> 129 (nearly identity - TRC NOT being applied?)");
        eprintln!("\nHYPOTHESIS: moxcms may be treating SM245B as if it has the same TRC as sRGB");
        eprintln!("Or: the linearize/gamma tables are canceling out incorrectly");
    }
}
