//! Deep analysis of ICC profiles that moxcms fails to parse
//!
//! Uses lcms2 to inspect profile structure and identify missing features.

use std::path::{Path, PathBuf};

fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Analyze profiles that moxcms fails to parse
#[test]
fn test_analyze_moxcms_failures() {
    eprintln!("\n=== DETAILED PROFILE ANALYSIS ===\n");

    // Key profiles that moxcms fails on but lcms2 handles
    let profiles_to_check = [
        "profiles/lcms2/ibm-t61.icc",
        "profiles/lcms2/new.icc",
        "profiles/qcms/lcms_samsung_syncmaster.icc",
        "profiles/skcms/color.org/sRGB_D65_MAT.icc",
        "profiles/skcms/color.org/sRGB_D65_colorimetric.icc",
        "profiles/skcms/color.org/sRGB_ISO22028.icc",
        "profiles/skcms/misc/AdobeColorSpin.icc",
        "profiles/skcms/misc/SM245B.icc",
        "profiles/skcms/mobile/Display_P3_LUT.icc",
    ];

    for profile_path in profiles_to_check {
        let full_path = testdata_dir().join(profile_path);
        if !full_path.exists() {
            eprintln!("SKIP: {} (not found)", profile_path);
            continue;
        }

        let data = std::fs::read(&full_path).unwrap();
        let filename = full_path.file_name().unwrap().to_string_lossy();

        eprintln!("=== {} ===", filename);
        eprintln!("  Size: {} bytes", data.len());

        // Parse with lcms2
        match lcms2::Profile::new_icc(&data) {
            Ok(profile) => {
                eprintln!("  lcms2: OK");
                eprintln!("    Color space: {:?}", profile.color_space());
                eprintln!("    PCS: {:?}", profile.pcs());
                eprintln!("    Device class: {:?}", profile.device_class());
                eprintln!("    Version: {:?}", profile.version());

                // Try to create a transform
                let srgb = lcms2::Profile::new_srgb();
                match lcms2::Transform::<[u8; 3], [u8; 3]>::new(
                    &profile,
                    lcms2::PixelFormat::RGB_8,
                    &srgb,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                ) {
                    Ok(_) => eprintln!("    Transform: OK"),
                    Err(e) => eprintln!("    Transform: FAILED ({:?})", e),
                }
            }
            Err(e) => {
                eprintln!("  lcms2: FAILED ({:?})", e);
            }
        }

        // Parse with moxcms
        match moxcms::ColorProfile::new_from_slice(&data) {
            Ok(profile) => {
                eprintln!("  moxcms: OK");
                eprintln!("    Color space: {:?}", profile.color_space);
                eprintln!("    PCS: {:?}", profile.pcs);
                eprintln!("    Class: {:?}", profile.profile_class);
            }
            Err(e) => {
                eprintln!("  moxcms: FAILED ({:?})", e);
            }
        }

        // Parse with skcms
        match skcms_sys::parse_icc_profile(&data) {
            Some(profile) => {
                eprintln!("  skcms: OK");
                eprintln!("    has_trc: {}", profile.has_trc);
                eprintln!("    has_toXYZD50: {}", profile.has_toXYZD50);
                eprintln!("    has_A2B: {}", profile.has_A2B);
                eprintln!("    has_B2A: {}", profile.has_B2A);
            }
            None => {
                eprintln!("  skcms: FAILED");
            }
        }

        // Parse with qcms
        match qcms::Profile::new_from_slice(&data, false) {
            Some(_) => eprintln!("  qcms: OK"),
            None => eprintln!("  qcms: FAILED"),
        }

        eprintln!();
    }
}

/// Analyze the ICC header of failing profiles
#[test]
fn test_analyze_icc_headers() {
    eprintln!("\n=== ICC HEADER ANALYSIS ===\n");

    let failing_profiles = [
        "profiles/lcms2/ibm-t61.icc",
        "profiles/lcms2/new.icc",
        "profiles/skcms/color.org/sRGB_D65_MAT.icc",
        "profiles/skcms/misc/SM245B.icc",
    ];

    for profile_path in failing_profiles {
        let full_path = testdata_dir().join(profile_path);
        if !full_path.exists() {
            continue;
        }

        let data = std::fs::read(&full_path).unwrap();
        let filename = full_path.file_name().unwrap().to_string_lossy();

        if data.len() < 128 {
            eprintln!("{}: Too small for ICC header", filename);
            continue;
        }

        eprintln!("=== {} ===", filename);

        // Parse ICC header manually
        let profile_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        eprintln!("  Profile size: {} (file: {})", profile_size, data.len());

        let cmm_type = String::from_utf8_lossy(&data[4..8]);
        eprintln!("  CMM type: '{}'", cmm_type.trim_matches('\0'));

        let version_major = data[8];
        let version_minor = (data[9] >> 4) & 0x0F;
        let version_patch = data[9] & 0x0F;
        eprintln!(
            "  Version: {}.{}.{}",
            version_major, version_minor, version_patch
        );

        let device_class = String::from_utf8_lossy(&data[12..16]);
        eprintln!("  Device class: '{}'", device_class.trim_matches('\0'));

        let color_space = String::from_utf8_lossy(&data[16..20]);
        eprintln!("  Color space: '{}'", color_space.trim_matches('\0'));

        let pcs = String::from_utf8_lossy(&data[20..24]);
        eprintln!("  PCS: '{}'", pcs.trim_matches('\0'));

        let rendering_intent = u32::from_be_bytes([data[64], data[65], data[66], data[67]]);
        eprintln!("  Rendering intent: {}", rendering_intent);

        // Tag count
        let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]);
        eprintln!("  Tag count: {}", tag_count);

        // List first few tags
        eprintln!("  Tags:");
        for i in 0..tag_count.min(15) as usize {
            let offset = 132 + i * 12;
            if offset + 12 > data.len() {
                break;
            }
            let sig = String::from_utf8_lossy(&data[offset..offset + 4]);
            let tag_offset = u32::from_be_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            let tag_size = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);
            eprintln!(
                "    '{}' @ offset {}, size {}",
                sig.trim_matches('\0'),
                tag_offset,
                tag_size
            );
        }

        eprintln!();
    }
}

/// Specifically investigate "Unknown parametric curve tag" errors
#[test]
fn test_analyze_parametric_curves() {
    eprintln!("\n=== PARAMETRIC CURVE ANALYSIS ===\n");

    // Profiles with MalformedTrcCurve errors
    let profiles = [
        "profiles/skcms/fuzz/b2a_no_clut.icc",
        "profiles/skcms/fuzz/b2a_too_few_output_channels.icc",
    ];

    for profile_path in profiles {
        let full_path = testdata_dir().join(profile_path);
        if !full_path.exists() {
            continue;
        }

        let data = std::fs::read(&full_path).unwrap();
        let filename = full_path.file_name().unwrap().to_string_lossy();

        eprintln!("=== {} ===", filename);

        // Parse with skcms to see what it finds
        if let Some(profile) = skcms_sys::parse_icc_profile(&data) {
            eprintln!("  skcms parsed successfully");
            eprintln!("    has_trc: {}", profile.has_trc);
            eprintln!("    has_A2B: {}", profile.has_A2B);
            eprintln!("    has_B2A: {}", profile.has_B2A);
        }

        // Check for 'para' (parametric curve) tags
        if data.len() > 132 {
            let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]);

            for i in 0..tag_count.min(50) as usize {
                let offset = 132 + i * 12;
                if offset + 12 > data.len() {
                    break;
                }
                let sig = &data[offset..offset + 4];

                // Look for TRC tags (rTRC, gTRC, bTRC, kTRC) or para tags
                if sig == b"rTRC" || sig == b"gTRC" || sig == b"bTRC" || sig == b"kTRC" {
                    let tag_offset = u32::from_be_bytes([
                        data[offset + 4],
                        data[offset + 5],
                        data[offset + 6],
                        data[offset + 7],
                    ]) as usize;
                    let tag_size = u32::from_be_bytes([
                        data[offset + 8],
                        data[offset + 9],
                        data[offset + 10],
                        data[offset + 11],
                    ]) as usize;

                    if tag_offset + 4 <= data.len() {
                        let type_sig = String::from_utf8_lossy(&data[tag_offset..tag_offset + 4]);
                        eprintln!(
                            "  {} tag at {}, size {}, type '{}'",
                            String::from_utf8_lossy(sig),
                            tag_offset,
                            tag_size,
                            type_sig.trim_matches('\0')
                        );

                        // If it's parametric, check the function type
                        if &data[tag_offset..tag_offset + 4] == b"para"
                            && tag_offset + 10 <= data.len()
                        {
                            let func_type =
                                u16::from_be_bytes([data[tag_offset + 8], data[tag_offset + 9]]);
                            eprintln!("    Parametric function type: {}", func_type);
                        }
                    }
                }
            }
        }

        eprintln!();
    }
}
