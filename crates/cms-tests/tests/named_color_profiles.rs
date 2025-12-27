//! Named Color Profile Tests
//!
//! Tests for ICC Named Color (nmcl/ncl2) profiles.
//! Named color profiles contain spot colors like Pantone, which can be
//! looked up by name and mapped to device-independent PCS values.

use std::path::Path;

/// Check if a profile is a named color profile by examining header bytes
fn is_named_color_profile(data: &[u8]) -> bool {
    if data.len() < 20 {
        return false;
    }
    // Profile class is at offset 12, should be 'nmcl' (0x6e6d636c)
    data[12..16] == [0x6e, 0x6d, 0x63, 0x6c]
}

/// Parse profile class from header
fn get_profile_class(data: &[u8]) -> Option<String> {
    if data.len() < 16 {
        return None;
    }
    Some(String::from_utf8_lossy(&data[12..16]).to_string())
}

#[test]
fn test_named_color_profile_structure() {
    eprintln!("\n=== Named Color Profile Structure ===\n");

    eprintln!("ICC Named Color Profile (nmcl class):");
    eprintln!("  - Profile class: 'nmcl' (0x6e6d636c)");
    eprintln!("  - Contains 'ncl2' tag (namedColor2Type)");
    eprintln!("  - Each color has:");
    eprintln!("    * Name (up to 32 chars)");
    eprintln!("    * Prefix/Suffix (e.g., 'PANTONE' + '123 C')");
    eprintln!("    * PCS values (Lab or XYZ)");
    eprintln!("    * Device colorant values (e.g., CMYK)");
    eprintln!();
    eprintln!("Use cases:");
    eprintln!("  - Spot colors (Pantone, HKS, etc.)");
    eprintln!("  - Corporate brand colors");
    eprintln!("  - Packaging inks");
    eprintln!();

    // Show ncl2 tag structure
    eprintln!("ncl2 Tag Structure (namedColor2Type):");
    eprintln!("  Offset  Size   Description");
    eprintln!("  0       4      Type signature ('ncl2')");
    eprintln!("  4       4      Reserved (0)");
    eprintln!("  8       4      Vendor flag");
    eprintln!("  12      4      Count of named colors");
    eprintln!("  16      4      Number of device coords");
    eprintln!("  20      32     Prefix for color names");
    eprintln!("  52      32     Suffix for color names");
    eprintln!("  84      var    Array of named color entries");
    eprintln!();
    eprintln!("Each named color entry:");
    eprintln!("  0       32     Color name (null-terminated)");
    eprintln!("  32      6      PCS coords (16-bit Lab or XYZ)");
    eprintln!("  38      var    Device colorant coords (16-bit each)");
}

#[test]
fn test_read_existing_named_colors_xml() {
    eprintln!("\n=== Reading Named Color XML Definitions ===\n");

    // Read the XML files we downloaded from ICC DemoIccMAX
    let xml_dir = Path::new("/home/lilith/oxcms/test-data/named-colors");

    if !xml_dir.exists() {
        eprintln!("  XML directory not found, skipping");
        return;
    }

    let mut found = 0;
    for entry in std::fs::read_dir(xml_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            found += 1;
            let filename = path.file_name().unwrap().to_string_lossy();
            eprintln!("  [{}] {}", found, filename);

            // Read and show basic structure
            if let Ok(content) = std::fs::read_to_string(&path) {
                // Count color definitions
                let color_count = content.matches("<NamedColor").count();
                let has_spectral = content.contains("SpectralReflectance");
                let has_tint = content.contains("Tint");
                let has_fluorescent = content.contains("Fluorescent");

                eprintln!("      Colors: {}", color_count);
                eprintln!(
                    "      Features: {}{}{}",
                    if has_spectral { "spectral " } else { "" },
                    if has_tint { "tints " } else { "" },
                    if has_fluorescent { "fluorescence " } else { "" }
                );
                eprintln!();
            }
        }
    }

    if found == 0 {
        eprintln!("  No XML files found");
    } else {
        eprintln!("  Found {} named color XML definitions", found);
        eprintln!();
        eprintln!("  Note: These are iccMAX v5 XML format files.");
        eprintln!("  They can be converted to .icc using iccFromXml tool.");
    }
}

#[test]
fn test_check_existing_profiles_for_named_colors() {
    eprintln!("\n=== Checking Existing Profiles for Named Colors ===\n");

    let profiles_dir = Path::new("/home/lilith/oxcms/testdata/profiles");

    if !profiles_dir.exists() {
        eprintln!("  Profiles directory not found");
        return;
    }

    let mut checked = 0;
    let mut named_color_profiles = 0;

    // Recursively find all .icc files
    fn check_dir(dir: &Path, checked: &mut usize, named: &mut usize) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    check_dir(&path, checked, named);
                } else if path.extension().map(|e| e == "icc").unwrap_or(false) {
                    *checked += 1;
                    if let Ok(data) = std::fs::read(&path) {
                        if is_named_color_profile(&data) {
                            *named += 1;
                            eprintln!("  FOUND: {}", path.display());
                        }
                    }
                }
            }
        }
    }

    check_dir(profiles_dir, &mut checked, &mut named_color_profiles);

    eprintln!();
    eprintln!("  Checked {} profiles", checked);
    eprintln!("  Found {} named color profiles", named_color_profiles);

    if named_color_profiles == 0 {
        eprintln!();
        eprintln!("  Note: Named color profiles (nmcl class) are rare.");
        eprintln!("  They're typically used for proprietary spot color systems");
        eprintln!("  like Pantone, which aren't freely available.");
    }
}

#[test]
fn test_profile_class_distribution() {
    eprintln!("\n=== Profile Class Distribution in Corpus ===\n");

    let profiles_dir = Path::new("/home/lilith/oxcms/testdata/profiles");

    if !profiles_dir.exists() {
        eprintln!("  Profiles directory not found");
        return;
    }

    use std::collections::HashMap;
    let mut class_counts: HashMap<String, usize> = HashMap::new();

    fn check_dir(dir: &Path, counts: &mut HashMap<String, usize>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    check_dir(&path, counts);
                } else if path.extension().map(|e| e == "icc").unwrap_or(false) {
                    if let Ok(data) = std::fs::read(&path) {
                        if let Some(class) = get_profile_class(&data) {
                            *counts.entry(class).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    check_dir(profiles_dir, &mut class_counts);

    eprintln!("  Profile classes found:");
    let mut sorted: Vec<_> = class_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    for (class, count) in sorted {
        let class_name = match class.as_str() {
            "mntr" => "Monitor/Display",
            "prtr" => "Output/Printer",
            "scnr" => "Input/Scanner",
            "link" => "Device Link",
            "spac" => "Color Space",
            "abst" => "Abstract",
            "nmcl" => "Named Color",
            _ => "Unknown",
        };
        eprintln!("    {} ({:4}): {:3} profiles", class, class_name, count);
    }
}
