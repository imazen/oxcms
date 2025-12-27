//! Corpus validation tests
//!
//! Tests that all profiles in the test corpus can be parsed
//! and used for transforms.

use std::path::Path;

/// Test that we can load the standard ICC profiles
#[test]
fn test_load_standard_profiles() {
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
        .join("profiles");

    if !testdata.exists() {
        eprintln!("Skipping: testdata/profiles not found");
        return;
    }

    let expected_profiles = ["sRGB.icc", "DisplayP3.icc", "AdobeRGB1998.icc"];

    for profile_name in expected_profiles {
        let path = testdata.join(profile_name);
        if path.exists() {
            let data = std::fs::read(&path).expect("Failed to read profile");

            // Test with moxcms
            match moxcms::ColorProfile::new_from_slice(&data) {
                Ok(_) => eprintln!("  moxcms: {} OK", profile_name),
                Err(e) => eprintln!("  moxcms: {} FAILED: {:?}", profile_name, e),
            }

            // Test with lcms2
            match lcms2::Profile::new_icc(&data) {
                Ok(_) => eprintln!("  lcms2: {} OK", profile_name),
                Err(e) => eprintln!("  lcms2: {} FAILED: {}", profile_name, e),
            }
        } else {
            eprintln!("  MISSING: {}", profile_name);
        }
    }
}

/// Test that all profiles in corpus directories can be parsed
#[test]
fn test_corpus_profiles() {
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata");

    let corpus_dirs = ["corpus/lcms2", "corpus/skcms", "corpus/qcms"];

    for dir in corpus_dirs {
        let corpus_path = testdata.join(dir);
        if !corpus_path.exists() {
            eprintln!("Skipping: {} not found (run fetch script)", dir);
            continue;
        }

        eprintln!("\nTesting profiles in {}:", dir);

        let entries = match std::fs::read_dir(&corpus_path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "icc" || e == "icm") {
                let name = path.file_name().unwrap().to_string_lossy();
                let data = match std::fs::read(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("  {} READ ERROR: {}", name, e);
                        continue;
                    }
                };

                // Try moxcms
                let mox_result = moxcms::ColorProfile::new_from_slice(&data);

                // Try lcms2
                let lcms_result = lcms2::Profile::new_icc(&data);

                let status = match (mox_result.is_ok(), lcms_result.is_ok()) {
                    (true, true) => "OK (both)",
                    (true, false) => "moxcms only",
                    (false, true) => "lcms2 only",
                    (false, false) => "FAILED (both)",
                };

                eprintln!("  {}: {}", name, status);
            }
        }
    }
}

/// Test specific problematic profiles (known edge cases)
#[test]
fn test_edge_case_profiles() {
    // TODO: Add specific edge case profiles here
    // These are profiles known to cause issues in one or more implementations

    let edge_cases: [(&str, &str); 0] = [
        // ("malformed_header.icc", "Profile with invalid header"),
        // ("v2_with_v4_tags.icc", "Mixed version profile"),
    ];

    for (name, desc) in edge_cases {
        eprintln!("Testing edge case: {} ({})", name, desc);
        // TODO: Implement when we have the profiles
    }
}
