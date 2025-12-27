//! Test corpus management
//!
//! Manages ICC profiles from various sources for testing.

use std::path::{Path, PathBuf};

/// Source of a test profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    /// From lcms2 testbed
    Lcms2,
    /// From skcms profiles directory
    Skcms,
    /// From qcms tests
    Qcms,
    /// From color.org
    ColorOrg,
    /// From moxcms tests
    Moxcms,
    /// Custom/local
    Custom,
}

/// A test profile from the corpus
#[derive(Debug)]
pub struct TestProfile {
    /// Profile name
    pub name: String,
    /// Source of the profile
    pub source: ProfileSource,
    /// Path to the profile file
    pub path: PathBuf,
    /// Profile description
    pub description: String,
    /// Expected color space
    pub color_space: String,
    /// ICC version
    pub version: u8,
}

/// Test corpus containing profiles from all sources
pub struct TestCorpus {
    profiles: Vec<TestProfile>,
    base_path: PathBuf,
}

impl TestCorpus {
    /// Create a new corpus from the testdata directory
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            profiles: Vec::new(),
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Load all profiles from the corpus
    pub fn load(&mut self) -> std::io::Result<()> {
        // Load from each source directory
        self.load_source(ProfileSource::Lcms2, "corpus/lcms2")?;
        self.load_source(ProfileSource::Skcms, "corpus/skcms")?;
        self.load_source(ProfileSource::Qcms, "corpus/qcms")?;
        self.load_source(ProfileSource::ColorOrg, "profiles")?;
        Ok(())
    }

    fn load_source(&mut self, source: ProfileSource, subdir: &str) -> std::io::Result<()> {
        let dir = self.base_path.join(subdir);
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "icc" || e == "icm") {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();

                self.profiles.push(TestProfile {
                    name: name.clone(),
                    source,
                    path,
                    description: format!("{} from {:?}", name, source),
                    color_space: "RGB".into(), // TODO: parse from profile
                    version: 4,                // TODO: parse from profile
                });
            }
        }
        Ok(())
    }

    /// Get all profiles
    pub fn profiles(&self) -> &[TestProfile] {
        &self.profiles
    }

    /// Get profiles from a specific source
    pub fn profiles_from(&self, source: ProfileSource) -> Vec<&TestProfile> {
        self.profiles
            .iter()
            .filter(|p| p.source == source)
            .collect()
    }

    /// Get profile by name
    pub fn get(&self, name: &str) -> Option<&TestProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }
}

/// Known test profiles that should be present
pub mod known_profiles {
    /// sRGB profiles
    pub const SRGB_V2: &str = "sRGB_v2";
    pub const SRGB_V4: &str = "sRGB_v4_ICC_preference";

    /// Display profiles
    pub const DISPLAY_P3: &str = "Display_P3";
    pub const ADOBE_RGB: &str = "AdobeRGB1998";
    pub const REC2020: &str = "Rec2020";

    /// CMYK profiles
    pub const FOGRA29: &str = "UncoatedFOGRA29";
    pub const FOGRA39: &str = "CoatedFOGRA39";
    pub const GRACOL: &str = "GRACoL2006_Coated1v2";
}
