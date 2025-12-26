//! Extended parity tests between moxcms (via oxcms) and lcms2
//!
//! These tests compare color transforms across multiple profiles and
//! document any differences using deltaE2000.

use cms_tests::accuracy::{compare_rgb_buffers, delta_e_2000, srgb_to_lab};
use moxcms::TransformExecutor;
use std::path::Path;

/// Test sRGB to Display P3 transform parity
#[test]
fn test_srgb_to_p3_parity() {
    // Create profiles for both implementations
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let mox_p3 = moxcms::ColorProfile::new_display_p3();

    // Test with a gradient of colors
    let test_colors: Vec<[u8; 3]> = (0..=255)
        .step_by(17)
        .flat_map(|r| {
            (0..=255)
                .step_by(51)
                .flat_map(move |g| (0..=255).step_by(51).map(move |b| [r as u8, g as u8, b as u8]))
        })
        .collect();

    // Create moxcms transform
    let mox_transform = mox_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &mox_p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("moxcms transform");

    // Transform with moxcms
    let mox_input: Vec<u8> = test_colors.iter().flat_map(|c| c.iter().copied()).collect();
    let mut mox_output = vec![0u8; mox_input.len()];

    mox_transform
        .transform(&mox_input, &mut mox_output)
        .expect("moxcms transform execute");

    // Verify transforms produce reasonable output
    // P3 has wider gamut so sRGB colors should shift inward
    let mut max_delta_e = 0.0f64;
    let mut total_delta_e = 0.0f64;
    let mut count = 0;

    for (i, chunk) in mox_output.chunks(3).enumerate() {
        let src = test_colors[i];
        let dst = [chunk[0], chunk[1], chunk[2]];

        let src_lab = srgb_to_lab(src[0], src[1], src[2]);
        let dst_lab = srgb_to_lab(dst[0], dst[1], dst[2]);

        let delta_e = delta_e_2000(src_lab, dst_lab);
        max_delta_e = max_delta_e.max(delta_e);
        total_delta_e += delta_e;
        count += 1;
    }

    let mean_delta_e = total_delta_e / count as f64;

    eprintln!("\nsRGB to P3 transform:");
    eprintln!("  Mean ΔE2000: {:.4}", mean_delta_e);
    eprintln!("  Max ΔE2000:  {:.4}", max_delta_e);
    eprintln!("  Sample count: {}", count);

    // sRGB to P3 should produce visible changes for saturated colors
    assert!(max_delta_e > 0.1, "Expected visible color difference for sRGB to P3");
}

/// Test that transforms are deterministic
#[test]
fn test_transform_determinism() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("transform");



    let input = [255u8, 128, 64];
    let mut output1 = [0u8; 3];
    let mut output2 = [0u8; 3];

    transform.transform(&input, &mut output1).unwrap();
    transform.transform(&input, &mut output2).unwrap();

    assert_eq!(output1, output2, "Transforms must be deterministic");
}

/// Test round-trip transform accuracy
#[test]
fn test_round_trip_accuracy() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let forward = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("forward transform");

    let backward = p3
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("backward transform");



    // Test with mid-gray (should round-trip well)
    let original = [128u8, 128, 128];
    let mut intermediate = [0u8; 3];
    let mut final_result = [0u8; 3];

    forward.transform(&original, &mut intermediate).unwrap();
    backward.transform(&intermediate, &mut final_result).unwrap();

    // Calculate round-trip error
    let orig_lab = srgb_to_lab(original[0], original[1], original[2]);
    let final_lab = srgb_to_lab(final_result[0], final_result[1], final_result[2]);
    let round_trip_error = delta_e_2000(orig_lab, final_lab);

    eprintln!("\nRound-trip accuracy (sRGB->P3->sRGB):");
    eprintln!("  Original:     {:?}", original);
    eprintln!("  Intermediate: {:?}", intermediate);
    eprintln!("  Final:        {:?}", final_result);
    eprintln!("  ΔE2000:       {:.4}", round_trip_error);

    // Round-trip should be accurate within 1 ΔE for gray
    assert!(
        round_trip_error < 1.0,
        "Round-trip error too high: {}",
        round_trip_error
    );
}

/// Test extreme color values
#[test]
fn test_extreme_values() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let transform = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("transform");



    let extreme_colors = [
        [0u8, 0, 0],       // Black
        [255, 255, 255],   // White
        [255, 0, 0],       // Pure red
        [0, 255, 0],       // Pure green
        [0, 0, 255],       // Pure blue
        [255, 255, 0],     // Yellow
        [0, 255, 255],     // Cyan
        [255, 0, 255],     // Magenta
    ];

    eprintln!("\nExtreme color transforms (sRGB -> P3):");
    for color in &extreme_colors {
        let mut output = [0u8; 3];
        transform.transform(color, &mut output).unwrap();

        let src_lab = srgb_to_lab(color[0], color[1], color[2]);
        let dst_lab = srgb_to_lab(output[0], output[1], output[2]);
        let delta_e = delta_e_2000(src_lab, dst_lab);

        eprintln!(
            "  {:?} -> {:?} (ΔE: {:.4})",
            color, output, delta_e
        );
    }
}

/// Test lcms2 sRGB identity matches moxcms
#[test]
fn test_lcms2_srgb_identity() {
    // Create lcms2 transform
    let lcms_srgb = lcms2::Profile::new_srgb();
    let lcms_transform = lcms2::Transform::new(
        &lcms_srgb,
        lcms2::PixelFormat::RGB_8,
        &lcms_srgb,
        lcms2::PixelFormat::RGB_8,
        lcms2::Intent::Perceptual,
    )
    .expect("lcms2 transform");

    // Create moxcms transform
    let mox_srgb = moxcms::ColorProfile::new_srgb();
    let mox_transform = mox_srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &mox_srgb,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("moxcms transform");

    // Test with full color cube
    let test_values: Vec<u8> = (0..=255).step_by(17).map(|v| v as u8).collect();
    let mut max_diff = 0i32;

    for &r in &test_values {
        for &g in &test_values {
            for &b in &test_values {
                let input = [r, g, b];

                // lcms2 transform
                let mut lcms_output = [0u8; 3];
                lcms_transform.transform_pixels(&input, &mut lcms_output);

                // moxcms transform
                let mut mox_output = [0u8; 3];
            
                mox_transform.transform(&input, &mut mox_output).unwrap();

                // Compare
                for i in 0..3 {
                    let diff = (lcms_output[i] as i32 - mox_output[i] as i32).abs();
                    max_diff = max_diff.max(diff);
                }
            }
        }
    }

    eprintln!("\nlcms2 vs moxcms sRGB identity:");
    eprintln!("  Max channel difference: {}", max_diff);

    // Both should produce identical sRGB identity transforms
    assert!(
        max_diff <= 1,
        "sRGB identity transforms should match within 1 level"
    );
}

/// Test with loaded ICC profile files
#[test]
fn test_loaded_profile_parity() {
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
        .join("profiles");

    let srgb_path = testdata.join("sRGB.icc");
    if !srgb_path.exists() {
        eprintln!("Skipping: sRGB.icc not found (run fetch script)");
        return;
    }

    let srgb_data = std::fs::read(&srgb_path).expect("read sRGB.icc");

    // Parse with both implementations
    let mox_profile = moxcms::ColorProfile::new_from_slice(&srgb_data);
    let lcms_profile = lcms2::Profile::new_icc(&srgb_data);

    match (&mox_profile, &lcms_profile) {
        (Ok(mox), Ok(lcms)) => {
            eprintln!("\nLoaded sRGB.icc:");
            eprintln!("  moxcms: parsed successfully");
            eprintln!("  lcms2:  parsed successfully");

            // Create identity transforms
            let mox_transform = mox
                .create_transform_8bit(
                    moxcms::Layout::Rgb,
                    mox,
                    moxcms::Layout::Rgb,
                    moxcms::TransformOptions::default(),
                )
                .expect("moxcms transform");

            let lcms_transform = lcms2::Transform::new(
                lcms,
                lcms2::PixelFormat::RGB_8,
                lcms,
                lcms2::PixelFormat::RGB_8,
                lcms2::Intent::Perceptual,
            )
            .expect("lcms2 transform");

            // Compare transforms
            let test_color = [200u8, 100, 50];
            let mut mox_output = [0u8; 3];
            let mut lcms_output = [0u8; 3];

        
            mox_transform.transform(&test_color, &mut mox_output).unwrap();
            lcms_transform.transform_pixels(&test_color, &mut lcms_output);

            eprintln!("  Input:  {:?}", test_color);
            eprintln!("  moxcms: {:?}", mox_output);
            eprintln!("  lcms2:  {:?}", lcms_output);

            let stats = compare_rgb_buffers(&mox_output, &lcms_output);
            eprintln!("  ΔE2000: {:.4}", stats.mean);
        }
        (Err(e1), Err(e2)) => {
            eprintln!("\nBoth failed to parse sRGB.icc:");
            eprintln!("  moxcms: {:?}", e1);
            eprintln!("  lcms2:  {}", e2);
        }
        (Ok(_), Err(e)) => {
            eprintln!("\nlcms2 failed to parse sRGB.icc: {}", e);
        }
        (Err(e), Ok(_)) => {
            eprintln!("\nmoxcms failed to parse sRGB.icc: {:?}", e);
        }
    }
}

/// Test 16-bit precision
#[test]
fn test_16bit_precision() {
    let srgb = moxcms::ColorProfile::new_srgb();
    let p3 = moxcms::ColorProfile::new_display_p3();

    let transform_8 = srgb
        .create_transform_8bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("8-bit transform");

    let transform_16 = srgb
        .create_transform_16bit(
            moxcms::Layout::Rgb,
            &p3,
            moxcms::Layout::Rgb,
            moxcms::TransformOptions::default(),
        )
        .expect("16-bit transform");



    // Test mid-gray at different bit depths
    let input_8 = [128u8, 128, 128];
    let input_16: [u16; 3] = [32768, 32768, 32768]; // Mid-gray in 16-bit

    let mut output_8 = [0u8; 3];
    let mut output_16 = [0u16; 3];

    transform_8.transform(&input_8, &mut output_8).unwrap();
    transform_16.transform(&input_16, &mut output_16).unwrap();

    // Convert 16-bit output to 8-bit for comparison
    let output_16_as_8: [u8; 3] = [
        (output_16[0] >> 8) as u8,
        (output_16[1] >> 8) as u8,
        (output_16[2] >> 8) as u8,
    ];

    eprintln!("\n16-bit vs 8-bit precision:");
    eprintln!("  8-bit input:  {:?}", input_8);
    eprintln!("  8-bit output: {:?}", output_8);
    eprintln!("  16-bit input: {:?}", input_16);
    eprintln!("  16-bit output: {:?}", output_16);
    eprintln!("  16->8 converted: {:?}", output_16_as_8);

    // Results should be very close
    for i in 0..3 {
        let diff = (output_8[i] as i32 - output_16_as_8[i] as i32).abs();
        assert!(
            diff <= 2,
            "16-bit and 8-bit results should be close: diff={} at channel {}",
            diff, i
        );
    }
}
