//! ICC Color Profile handling
//!
//! This module provides ICC profile parsing and manipulation.
//! It wraps moxcms::ColorProfile with additional validation.

use crate::{Error, Result};

/// ICC Color Profile
///
/// Represents a parsed ICC color profile. Supports ICC v2 and v4 profiles.
///
/// This is a thin wrapper around `moxcms::ColorProfile` that provides
/// additional validation and a stable API surface.
#[derive(Debug, Clone)]
pub struct ColorProfile {
    inner: moxcms::ColorProfile,
}

impl ColorProfile {
    /// Create a profile from raw ICC data
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let inner = moxcms::ColorProfile::new_from_slice(data)
            .map_err(|e| Error::ProfileParse(format!("{:?}", e)))?;
        Ok(Self { inner })
    }

    /// Create a built-in sRGB profile
    pub fn new_srgb() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_srgb(),
        }
    }

    /// Create a built-in Display P3 profile
    pub fn new_display_p3() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_display_p3(),
        }
    }

    /// Create a built-in Adobe RGB (1998) profile
    pub fn new_adobe_rgb() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_adobe_rgb(),
        }
    }

    /// Create a built-in BT.2020 profile
    pub fn new_bt2020() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_bt2020(),
        }
    }

    /// Create a built-in BT.709 profile (same primaries as sRGB)
    pub fn new_bt709() -> Self {
        // BT.709 uses same primaries as sRGB
        Self {
            inner: moxcms::ColorProfile::new_srgb(),
        }
    }

    /// Create a grayscale profile with specific gamma
    pub fn new_gray_with_gamma(gamma: f32) -> Self {
        Self {
            inner: moxcms::ColorProfile::new_gray_with_gamma(gamma),
        }
    }

    /// Create a DCI-P3 profile (theatrical cinema)
    pub fn new_dci_p3() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_dci_p3(),
        }
    }

    /// Create a ProPhoto RGB profile (wide gamut)
    pub fn new_pro_photo_rgb() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_pro_photo_rgb(),
        }
    }

    /// Create a Display P3 PQ profile (HDR)
    pub fn new_display_p3_pq() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_display_p3_pq(),
        }
    }

    /// Create a BT.2020 PQ profile (HDR)
    pub fn new_bt2020_pq() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_bt2020_pq(),
        }
    }

    /// Create a BT.2020 HLG profile (HDR)
    pub fn new_bt2020_hlg() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_bt2020_hlg(),
        }
    }

    /// Create a Lab profile (CIELAB D50)
    pub fn new_lab() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_lab(),
        }
    }

    /// Create an ACES 2065-1 linear profile (film/VFX)
    pub fn new_aces_linear() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_aces_aces_2065_1_linear(),
        }
    }

    /// Create an ACEScg linear profile (film/VFX)
    pub fn new_aces_cg() -> Self {
        Self {
            inner: moxcms::ColorProfile::new_aces_cg_linear(),
        }
    }

    /// Create a profile from CICP parameters (commonly used in video)
    pub fn from_cicp(cicp: moxcms::CicpProfile) -> Self {
        Self {
            inner: moxcms::ColorProfile::new_from_cicp(cicp),
        }
    }

    /// Get the profile's color space
    pub fn color_space(&self) -> moxcms::DataColorSpace {
        self.inner.color_space
    }

    /// Get the profile connection space (PCS)
    pub fn pcs(&self) -> moxcms::DataColorSpace {
        self.inner.pcs
    }

    /// Get the profile version
    pub fn version(&self) -> moxcms::ProfileVersion {
        self.inner.version()
    }

    /// Get the profile class
    pub fn profile_class(&self) -> moxcms::ProfileClass {
        self.inner.profile_class
    }

    /// Get the rendering intent
    pub fn rendering_intent(&self) -> moxcms::RenderingIntent {
        self.inner.rendering_intent
    }

    /// Check if this is a matrix-shaper profile
    pub fn is_matrix_shaper(&self) -> bool {
        self.inner.is_matrix_shaper()
    }

    /// Get the colorant matrix (for RGB profiles)
    pub fn colorant_matrix(&self) -> moxcms::Matrix3d {
        self.inner.colorant_matrix()
    }

    /// Get the white point
    pub fn white_point(&self) -> moxcms::Xyzd {
        self.inner.white_point
    }

    /// Get description text if available
    pub fn description(&self) -> Option<String> {
        self.inner.description.as_ref().map(|text| match text {
            moxcms::ProfileText::PlainString(s) => s.clone(),
            moxcms::ProfileText::Localizable(locs) => {
                locs.first().map(|l| l.value.clone()).unwrap_or_default()
            }
            moxcms::ProfileText::Description(desc) => desc.ascii_string.clone(),
        })
    }

    /// Get copyright text if available
    pub fn copyright(&self) -> Option<String> {
        self.inner.copyright.as_ref().map(|text| match text {
            moxcms::ProfileText::PlainString(s) => s.clone(),
            moxcms::ProfileText::Localizable(locs) => {
                locs.first().map(|l| l.value.clone()).unwrap_or_default()
            }
            moxcms::ProfileText::Description(desc) => desc.ascii_string.clone(),
        })
    }

    /// Access the inner moxcms profile
    pub fn inner(&self) -> &moxcms::ColorProfile {
        &self.inner
    }

    /// Create a transform between this profile and another
    pub fn create_transform_8bit(
        &self,
        src_layout: crate::Layout,
        dst_profile: &ColorProfile,
        dst_layout: crate::Layout,
        options: crate::TransformOptions,
    ) -> Result<crate::Transform> {
        crate::Transform::new_8bit(self, src_layout, dst_profile, dst_layout, options)
    }

    /// Create a 16-bit transform between this profile and another
    pub fn create_transform_16bit(
        &self,
        src_layout: crate::Layout,
        dst_profile: &ColorProfile,
        dst_layout: crate::Layout,
        options: crate::TransformOptions,
    ) -> Result<crate::Transform> {
        crate::Transform::new_16bit(self, src_layout, dst_profile, dst_layout, options)
    }

    /// Create a floating-point transform between this profile and another
    pub fn create_transform_f32(
        &self,
        src_layout: crate::Layout,
        dst_profile: &ColorProfile,
        dst_layout: crate::Layout,
        options: crate::TransformOptions,
    ) -> Result<crate::Transform> {
        crate::Transform::new_f32(self, src_layout, dst_profile, dst_layout, options)
    }
}

impl From<moxcms::ColorProfile> for ColorProfile {
    fn from(inner: moxcms::ColorProfile) -> Self {
        Self { inner }
    }
}

impl AsRef<moxcms::ColorProfile> for ColorProfile {
    fn as_ref(&self) -> &moxcms::ColorProfile {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_profile() {
        let profile = ColorProfile::new_srgb();
        assert_eq!(profile.color_space(), moxcms::DataColorSpace::Rgb);
        assert!(profile.is_matrix_shaper());
    }

    #[test]
    fn test_display_p3_profile() {
        let profile = ColorProfile::new_display_p3();
        assert_eq!(profile.color_space(), moxcms::DataColorSpace::Rgb);
        assert!(profile.is_matrix_shaper());
    }

    #[test]
    fn test_reject_small_profile() {
        let small_data = [0u8; 64];
        assert!(ColorProfile::from_bytes(&small_data).is_err());
    }

    #[test]
    fn test_lab_profile() {
        let profile = ColorProfile::new_lab();
        assert_eq!(profile.color_space(), moxcms::DataColorSpace::Lab);
    }

    #[test]
    fn test_pro_photo_rgb_profile() {
        let profile = ColorProfile::new_pro_photo_rgb();
        assert_eq!(profile.color_space(), moxcms::DataColorSpace::Rgb);
        assert!(profile.is_matrix_shaper());
    }

    #[test]
    fn test_dci_p3_profile() {
        let profile = ColorProfile::new_dci_p3();
        assert_eq!(profile.color_space(), moxcms::DataColorSpace::Rgb);
        assert!(profile.is_matrix_shaper());
    }

    #[test]
    fn test_hdr_profiles() {
        let p3_pq = ColorProfile::new_display_p3_pq();
        assert_eq!(p3_pq.color_space(), moxcms::DataColorSpace::Rgb);

        let bt2020_pq = ColorProfile::new_bt2020_pq();
        assert_eq!(bt2020_pq.color_space(), moxcms::DataColorSpace::Rgb);

        let bt2020_hlg = ColorProfile::new_bt2020_hlg();
        assert_eq!(bt2020_hlg.color_space(), moxcms::DataColorSpace::Rgb);
    }

    #[test]
    fn test_aces_profiles() {
        let aces = ColorProfile::new_aces_linear();
        assert_eq!(aces.color_space(), moxcms::DataColorSpace::Rgb);

        let aces_cg = ColorProfile::new_aces_cg();
        assert_eq!(aces_cg.color_space(), moxcms::DataColorSpace::Rgb);
    }

    #[test]
    fn test_grayscale_profile() {
        let gray = ColorProfile::new_gray_with_gamma(2.2);
        assert_eq!(gray.color_space(), moxcms::DataColorSpace::Gray);
    }
}
