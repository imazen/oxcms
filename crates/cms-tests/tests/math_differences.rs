//! Exhaustive documentation of math differences between CMS implementations
//!
//! This test suite documents every mathematical difference between
//! lcms2, moxcms, skcms, and qcms.
//!
//! These are NOT failures - they are documented differences.
//! Each difference should be investigated and either:
//! 1. Fixed to match reference
//! 2. Documented with justification for why we differ

use cms_tests::accuracy::{delta_e_2000, srgb_to_lab};
use cms_tests::patterns::{TestPattern, generate_pattern};
use cms_tests::reference::{transform_lcms2_srgb, transform_moxcms_srgb};
use std::collections::HashMap;

/// Detailed difference record
#[derive(Debug)]
struct DifferenceRecord {
    input_rgb: [u8; 3],
    moxcms_output: [u8; 3],
    lcms2_output: [u8; 3],
    delta_e: f64,
}

/// Find and document all differences between moxcms and lcms2
fn find_differences(input: &[u8]) -> Vec<DifferenceRecord> {
    let moxcms_output = transform_moxcms_srgb(input).expect("moxcms failed");
    let lcms2_output = transform_lcms2_srgb(input).expect("lcms2 failed");

    let mut differences = Vec::new();

    for i in 0..(input.len() / 3) {
        let idx = i * 3;
        let input_rgb = [input[idx], input[idx + 1], input[idx + 2]];
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
            let lab_mox = srgb_to_lab(mox[0], mox[1], mox[2]);
            let lab_lcms = srgb_to_lab(lcms[0], lcms[1], lcms[2]);
            let de = delta_e_2000(lab_mox, lab_lcms);

            differences.push(DifferenceRecord {
                input_rgb,
                moxcms_output: mox,
                lcms2_output: lcms,
                delta_e: de,
            });
        }
    }

    differences
}

/// Test all 256 grayscale values
#[test]
fn document_grayscale_differences() {
    let mut input = vec![0u8; 256 * 3];
    for i in 0..256 {
        input[i * 3] = i as u8;
        input[i * 3 + 1] = i as u8;
        input[i * 3 + 2] = i as u8;
    }

    let differences = find_differences(&input);

    if !differences.is_empty() {
        eprintln!("\n=== GRAYSCALE DIFFERENCES ===");
        eprintln!("Total: {} out of 256 values differ", differences.len());
        for d in &differences {
            eprintln!(
                "  Gray {} -> moxcms={:?}, lcms2={:?}, deltaE={:.4}",
                d.input_rgb[0], d.moxcms_output, d.lcms2_output, d.delta_e
            );
        }
    }

    // This test documents but doesn't fail
    // All differences should be < 1.0 deltaE (imperceptible)
    for d in &differences {
        assert!(
            d.delta_e < 1.0,
            "Grayscale {} has perceptible difference: deltaE={:.4}",
            d.input_rgb[0],
            d.delta_e
        );
    }
}

/// Test primary and secondary colors
#[test]
fn document_primary_color_differences() {
    let primaries: [[u8; 3]; 8] = [
        [0, 0, 0],       // Black
        [255, 0, 0],     // Red
        [0, 255, 0],     // Green
        [0, 0, 255],     // Blue
        [255, 255, 0],   // Yellow
        [255, 0, 255],   // Magenta
        [0, 255, 255],   // Cyan
        [255, 255, 255], // White
    ];

    let mut input = Vec::new();
    for p in &primaries {
        input.extend_from_slice(p);
    }

    let differences = find_differences(&input);

    if !differences.is_empty() {
        eprintln!("\n=== PRIMARY COLOR DIFFERENCES ===");
        for d in &differences {
            let name = match d.input_rgb {
                [0, 0, 0] => "Black",
                [255, 0, 0] => "Red",
                [0, 255, 0] => "Green",
                [0, 0, 255] => "Blue",
                [255, 255, 0] => "Yellow",
                [255, 0, 255] => "Magenta",
                [0, 255, 255] => "Cyan",
                [255, 255, 255] => "White",
                _ => "Unknown",
            };
            eprintln!(
                "  {} {:?} -> moxcms={:?}, lcms2={:?}, deltaE={:.4}",
                name, d.input_rgb, d.moxcms_output, d.lcms2_output, d.delta_e
            );
        }
    }

    // All primary colors must match exactly
    assert!(
        differences.is_empty(),
        "Primary colors should match exactly, found {} differences",
        differences.len()
    );
}

/// Test the full 256^3 color cube (sampling)
#[test]
fn document_color_cube_sample() {
    // Sample every 16th value to keep test fast
    let step = 16;
    let mut input = Vec::new();

    for r in (0..=255).step_by(step) {
        for g in (0..=255).step_by(step) {
            for b in (0..=255).step_by(step) {
                input.push(r as u8);
                input.push(g as u8);
                input.push(b as u8);
            }
        }
    }

    let differences = find_differences(&input);
    let total_samples = input.len() / 3;

    eprintln!("\n=== COLOR CUBE SAMPLE (step={}) ===", step);
    eprintln!("Total samples: {}", total_samples);
    eprintln!(
        "Differences: {} ({:.2}%)",
        differences.len(),
        100.0 * differences.len() as f64 / total_samples as f64
    );

    if !differences.is_empty() {
        // Categorize by deltaE magnitude
        let mut by_magnitude: HashMap<&str, usize> = HashMap::new();
        for d in &differences {
            let category = if d.delta_e < 0.1 {
                "< 0.1 (invisible)"
            } else if d.delta_e < 0.5 {
                "0.1-0.5 (barely visible)"
            } else if d.delta_e < 1.0 {
                "0.5-1.0 (threshold)"
            } else {
                "> 1.0 (VISIBLE)"
            };
            *by_magnitude.entry(category).or_insert(0) += 1;
        }

        eprintln!("By magnitude:");
        for (cat, count) in &by_magnitude {
            eprintln!("  {}: {}", cat, count);
        }

        // Show worst cases
        let mut worst: Vec<_> = differences.iter().collect();
        worst.sort_by(|a, b| b.delta_e.partial_cmp(&a.delta_e).unwrap());

        eprintln!("Worst 5 differences:");
        for d in worst.iter().take(5) {
            eprintln!(
                "  {:?} -> moxcms={:?}, lcms2={:?}, deltaE={:.4}",
                d.input_rgb, d.moxcms_output, d.lcms2_output, d.delta_e
            );
        }
    }

    // Fail if any difference is perceptible
    let max_delta_e = differences.iter().map(|d| d.delta_e).fold(0.0, f64::max);

    assert!(
        max_delta_e < 1.0,
        "Max deltaE = {:.4} exceeds perceptibility threshold",
        max_delta_e
    );
}

/// Generate markdown report of all differences
#[test]
fn generate_difference_report() {
    // This test generates docs/MATH_DIFFERENCES.md content
    // Run with: cargo test generate_difference_report -- --nocapture

    eprintln!("\n# Math Differences: moxcms vs lcms2\n");
    eprintln!("Generated: {}\n", chrono_lite_now());
    eprintln!("## Summary\n");
    eprintln!("| Category | Samples | Differences | Max ΔE |");
    eprintln!("|----------|---------|-------------|--------|");

    // Test each category
    let categories = [
        (
            "Grayscale",
            generate_pattern(TestPattern::Grayscale, 256, 1),
        ),
        ("Primaries", {
            let mut v = Vec::new();
            for c in &[
                [0u8, 0, 0],
                [255, 0, 0],
                [0, 255, 0],
                [0, 0, 255],
                [255, 255, 0],
                [255, 0, 255],
                [0, 255, 255],
                [255, 255, 255],
            ] {
                v.extend_from_slice(c);
            }
            v
        }),
        (
            "Skin tones",
            generate_pattern(TestPattern::SkinTones, 64, 1),
        ),
        (
            "Gamut boundary",
            generate_pattern(TestPattern::GamutBoundary, 64, 1),
        ),
        (
            "Random (seed 42)",
            generate_pattern(TestPattern::Random(42), 100, 1),
        ),
    ];

    for (name, input) in categories {
        let diffs = find_differences(&input);
        let samples = input.len() / 3;
        let max_de = diffs.iter().map(|d| d.delta_e).fold(0.0, f64::max);

        eprintln!(
            "| {} | {} | {} | {:.4} |",
            name,
            samples,
            diffs.len(),
            max_de
        );
    }

    eprintln!("\n## Details\n");
    eprintln!("See individual test output for detailed differences.");
}

fn chrono_lite_now() -> String {
    // Simplified date without chrono dependency
    "2025-12-25".to_string()
}
