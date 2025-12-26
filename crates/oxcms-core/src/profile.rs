//! ICC Color Profile handling
//!
//! This module provides ICC profile parsing and manipulation.
//! Currently a stub - implementation will be ported from moxcms.

use crate::{Error, Result};

/// ICC Color Profile
///
/// Represents a parsed ICC color profile. Supports ICC v2 and v4 profiles.
#[derive(Debug, Clone)]
pub struct ColorProfile {
    /// Raw profile data
    data: Vec<u8>,
    /// Profile version (2 or 4)
    version: u8,
    /// Color space of the profile
    color_space: ColorSpace,
    /// Profile connection space
    pcs: ProfileConnectionSpace,
}

/// Color space type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorSpace {
    /// RGB color space
    Rgb,
    /// CMYK color space
    Cmyk,
    /// Grayscale
    Gray,
    /// CIE L*a*b*
    Lab,
    /// CIE XYZ
    Xyz,
    /// Unknown color space
    Unknown(u32),
}

/// Profile connection space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileConnectionSpace {
    /// CIE XYZ
    Xyz,
    /// CIE L*a*b*
    Lab,
}

impl ColorProfile {
    /// Create a profile from raw ICC data
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 128 {
            return Err(Error::ProfileParse("Profile too small".into()));
        }

        // TODO: Implement full ICC parsing
        // For now, just store the data
        Ok(Self {
            data: data.to_vec(),
            version: 4,
            color_space: ColorSpace::Rgb,
            pcs: ProfileConnectionSpace::Xyz,
        })
    }

    /// Create a built-in sRGB profile
    pub fn srgb() -> Self {
        // TODO: Implement proper sRGB profile
        Self {
            data: Vec::new(),
            version: 4,
            color_space: ColorSpace::Rgb,
            pcs: ProfileConnectionSpace::Xyz,
        }
    }

    /// Get the profile's color space
    pub fn color_space(&self) -> ColorSpace {
        self.color_space
    }

    /// Get the profile version
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Get the raw profile data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_profile() {
        let profile = ColorProfile::srgb();
        assert_eq!(profile.color_space(), ColorSpace::Rgb);
    }

    #[test]
    fn test_reject_small_profile() {
        let small_data = [0u8; 64];
        assert!(ColorProfile::from_bytes(&small_data).is_err());
    }
}
