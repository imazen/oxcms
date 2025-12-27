//! Profile Version Check
//!
//! Check if the profile version affects how colorants should be interpreted.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Check SM245B profile version and characteristics
#[test]
fn check_profile_version() {
    eprintln!("\n=== Profile Version Check ===\n");

    let profiles_dir = testdata_dir().join("profiles");
    let profile_path = profiles_dir.join("skcms/misc/SM245B.icc");

    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let data = std::fs::read(&profile_path).unwrap();
    let profile = moxcms::ColorProfile::new_from_slice(&data).unwrap();

    eprintln!("SM245B profile info:");
    eprintln!("  Version: {:?}", profile.version());
    eprintln!("  Profile class: {:?}", profile.profile_class);
    eprintln!("  Color space: {:?}", profile.color_space);
    eprintln!("  PCS: {:?}", profile.pcs);
    eprintln!("  Is matrix-shaper: {}", profile.is_matrix_shaper());

    // Compare with sRGB
    let srgb = moxcms::ColorProfile::new_srgb();
    eprintln!("\nsRGB profile info:");
    eprintln!("  Version: {:?}", srgb.version());
    eprintln!("  Profile class: {:?}", srgb.profile_class);

    // Check raw bytes for profile header
    eprintln!("\n=== Raw Profile Header ===");
    if data.len() >= 128 {
        let version_major = data[8];
        let version_minor = data[9];
        eprintln!("Profile version (raw): {}.{}", version_major, version_minor >> 4);

        // Profile/Device class at offset 12-15
        let class = &data[12..16];
        eprintln!("Profile class (raw): {:?}", std::str::from_utf8(class).unwrap_or("?"));

        // Color space at offset 16-19
        let color_space = &data[16..20];
        eprintln!("Color space (raw): {:?}", std::str::from_utf8(color_space).unwrap_or("?"));

        // PCS at offset 20-23
        let pcs = &data[20..24];
        eprintln!("PCS (raw): {:?}", std::str::from_utf8(pcs).unwrap_or("?"));
    }

    // Diagnosis
    eprintln!("\n=== Diagnosis ===");
    eprintln!("SM245B colorants sum to ~D65, but white point tag is D50.");
    eprintln!("This is common in V2 profiles where chromatic adaptation");
    eprintln!("may not be applied to the colorants.");
    eprintln!("");
    eprintln!("For V2 profiles:");
    eprintln!("  - Colorants may be in native (device) white point space");
    eprintln!("  - CMM should apply chromatic adaptation if needed");
    eprintln!("");
    eprintln!("For V4 profiles:");
    eprintln!("  - Colorants MUST be adapted to D50 PCS");
    eprintln!("  - No additional adaptation needed");
    eprintln!("");
    eprintln!("skcms appears to detect this and apply chromatic adaptation.");
    eprintln!("moxcms is NOT applying chromatic adaptation, treating colorants as D50.");
}

/// Test with a known V4 profile to see if the behavior is different
#[test]
fn test_v4_profile() {
    eprintln!("\n=== V4 Profile Test ===\n");

    let profiles_dir = testdata_dir().join("profiles");

    // Try to find a V4 profile
    let v4_profiles = [
        "skcms/color.org/sRGB_v4_ICC_preference.icc",
        "skcms/mobile/sRGB_parametric.icc",
    ];

    for profile_rel in v4_profiles {
        let profile_path = profiles_dir.join(profile_rel);
        if !profile_path.exists() {
            continue;
        }

        let data = std::fs::read(&profile_path).unwrap();
        if data.len() < 128 {
            continue;
        }

        let version_major = data[8];
        eprintln!("{}: V{}.x", profile_rel, version_major);

        if let Ok(profile) = moxcms::ColorProfile::new_from_slice(&data) {
            eprintln!("  White point: {:?}", profile.white_point);

            // Check colorant sum
            let sum_x = profile.red_colorant.x + profile.green_colorant.x + profile.blue_colorant.x;
            let sum_y = profile.red_colorant.y + profile.green_colorant.y + profile.blue_colorant.y;
            let sum_z = profile.red_colorant.z + profile.green_colorant.z + profile.blue_colorant.z;
            eprintln!("  Colorants sum: [{:.4}, {:.4}, {:.4}]", sum_x, sum_y, sum_z);
            eprintln!("  White point: [{:.4}, {:.4}, {:.4}]", profile.white_point.x, profile.white_point.y, profile.white_point.z);

            let match_wp = (sum_x - profile.white_point.x).abs() < 0.01
                && (sum_y - profile.white_point.y).abs() < 0.01
                && (sum_z - profile.white_point.z).abs() < 0.01;

            if match_wp {
                eprintln!("  => Colorants match white point (correctly adapted)");
            } else {
                eprintln!("  => Colorants do NOT match white point!");
            }
        }
        eprintln!();
    }
}
