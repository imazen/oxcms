//! Color Transform operations
//!
//! This module provides color space transformations between profiles.
//! It wraps moxcms transforms with additional validation.

use crate::profile::ColorProfile;
use crate::{Error, Result};
use moxcms::TransformExecutor;

/// Rendering intent for color transformations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderingIntent {
    /// Perceptual rendering intent - best for photographic images
    #[default]
    Perceptual,
    /// Relative colorimetric - preserves in-gamut colors, clips out-of-gamut
    RelativeColorimetric,
    /// Saturation - maintains saturation, may shift hue
    Saturation,
    /// Absolute colorimetric - preserves white point
    AbsoluteColorimetric,
}

impl From<RenderingIntent> for moxcms::RenderingIntent {
    fn from(intent: RenderingIntent) -> Self {
        match intent {
            RenderingIntent::Perceptual => moxcms::RenderingIntent::Perceptual,
            RenderingIntent::RelativeColorimetric => moxcms::RenderingIntent::RelativeColorimetric,
            RenderingIntent::Saturation => moxcms::RenderingIntent::Saturation,
            RenderingIntent::AbsoluteColorimetric => moxcms::RenderingIntent::AbsoluteColorimetric,
        }
    }
}

impl From<moxcms::RenderingIntent> for RenderingIntent {
    fn from(intent: moxcms::RenderingIntent) -> Self {
        match intent {
            moxcms::RenderingIntent::Perceptual => RenderingIntent::Perceptual,
            moxcms::RenderingIntent::RelativeColorimetric => RenderingIntent::RelativeColorimetric,
            moxcms::RenderingIntent::Saturation => RenderingIntent::Saturation,
            moxcms::RenderingIntent::AbsoluteColorimetric => RenderingIntent::AbsoluteColorimetric,
        }
    }
}

/// Pixel layout for transforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// RGB, 3 channels
    Rgb,
    /// RGBA, 4 channels (alpha preserved)
    Rgba,
    /// Grayscale, 1 channel
    Gray,
    /// Grayscale + Alpha, 2 channels
    GrayAlpha,
}

impl Layout {
    /// Get number of channels for this layout
    pub fn channels(&self) -> usize {
        match self {
            Layout::Rgb => 3,
            Layout::Rgba => 4,
            Layout::Gray => 1,
            Layout::GrayAlpha => 2,
        }
    }

    /// Check if layout has alpha channel
    pub fn has_alpha(&self) -> bool {
        matches!(self, Layout::Rgba | Layout::GrayAlpha)
    }
}

impl From<Layout> for moxcms::Layout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Rgb => moxcms::Layout::Rgb,
            Layout::Rgba => moxcms::Layout::Rgba,
            Layout::Gray => moxcms::Layout::Gray,
            Layout::GrayAlpha => moxcms::Layout::GrayAlpha,
        }
    }
}

impl From<moxcms::Layout> for Layout {
    fn from(layout: moxcms::Layout) -> Self {
        match layout {
            moxcms::Layout::Rgb => Layout::Rgb,
            moxcms::Layout::Rgba => Layout::Rgba,
            moxcms::Layout::Gray => Layout::Gray,
            moxcms::Layout::GrayAlpha => Layout::GrayAlpha,
            _ => Layout::Rgba, // Multi-ink layouts map to RGBA
        }
    }
}

/// Options for transform creation
#[derive(Debug, Clone, Copy, Default)]
pub struct TransformOptions {
    /// Rendering intent
    pub intent: RenderingIntent,
    /// Enable black point compensation (not yet implemented)
    pub black_point_compensation: bool,
    /// Use CICP transfer functions when available
    pub allow_use_cicp_transfer: bool,
    /// Prefer fixed-point math for performance
    pub prefer_fixed_point: bool,
}

impl From<TransformOptions> for moxcms::TransformOptions {
    fn from(opts: TransformOptions) -> Self {
        moxcms::TransformOptions {
            rendering_intent: opts.intent.into(),
            allow_use_cicp_transfer: opts.allow_use_cicp_transfer,
            prefer_fixed_point: opts.prefer_fixed_point,
            ..Default::default()
        }
    }
}

/// A color transform between two profiles
///
/// Transforms are created from a source and destination profile,
/// and can then be applied to pixel data.
pub struct Transform {
    inner: TransformInner,
    src_layout: Layout,
    dst_layout: Layout,
}

enum TransformInner {
    U8(Box<moxcms::Transform8BitExecutor>),
    U16(Box<moxcms::Transform16BitExecutor>),
    F32(Box<moxcms::TransformF32BitExecutor>),
}

impl Transform {
    /// Create a new 8-bit transform
    pub fn new_8bit(
        src_profile: &ColorProfile,
        src_layout: Layout,
        dst_profile: &ColorProfile,
        dst_layout: Layout,
        options: TransformOptions,
    ) -> Result<Self> {
        let inner = src_profile
            .inner()
            .create_transform_8bit(
                src_layout.into(),
                dst_profile.inner(),
                dst_layout.into(),
                options.into(),
            )
            .map_err(|e| Error::Transform(format!("{:?}", e)))?;

        Ok(Self {
            inner: TransformInner::U8(inner),
            src_layout,
            dst_layout,
        })
    }

    /// Create a new 16-bit transform
    pub fn new_16bit(
        src_profile: &ColorProfile,
        src_layout: Layout,
        dst_profile: &ColorProfile,
        dst_layout: Layout,
        options: TransformOptions,
    ) -> Result<Self> {
        let inner = src_profile
            .inner()
            .create_transform_16bit(
                src_layout.into(),
                dst_profile.inner(),
                dst_layout.into(),
                options.into(),
            )
            .map_err(|e| Error::Transform(format!("{:?}", e)))?;

        Ok(Self {
            inner: TransformInner::U16(inner),
            src_layout,
            dst_layout,
        })
    }

    /// Create a new 32-bit floating point transform
    pub fn new_f32(
        src_profile: &ColorProfile,
        src_layout: Layout,
        dst_profile: &ColorProfile,
        dst_layout: Layout,
        options: TransformOptions,
    ) -> Result<Self> {
        let inner = src_profile
            .inner()
            .create_transform_f32(
                src_layout.into(),
                dst_profile.inner(),
                dst_layout.into(),
                options.into(),
            )
            .map_err(|e| Error::Transform(format!("{:?}", e)))?;

        Ok(Self {
            inner: TransformInner::F32(inner),
            src_layout,
            dst_layout,
        })
    }

    /// Get source layout
    pub fn src_layout(&self) -> Layout {
        self.src_layout
    }

    /// Get destination layout
    pub fn dst_layout(&self) -> Layout {
        self.dst_layout
    }

    /// Transform 8-bit pixel data
    pub fn transform(&self, src: &[u8], dst: &mut [u8]) -> Result<()> {
        match &self.inner {
            TransformInner::U8(t) => t
                .transform(src, dst)
                .map_err(|e| Error::Transform(format!("{:?}", e))),
            _ => Err(Error::Transform("Wrong bit depth for transform".into())),
        }
    }

    /// Transform 16-bit pixel data
    pub fn transform_u16(&self, src: &[u16], dst: &mut [u16]) -> Result<()> {
        match &self.inner {
            TransformInner::U16(t) => t
                .transform(src, dst)
                .map_err(|e| Error::Transform(format!("{:?}", e))),
            _ => Err(Error::Transform("Wrong bit depth for transform".into())),
        }
    }

    /// Transform 32-bit floating point pixel data
    pub fn transform_f32(&self, src: &[f32], dst: &mut [f32]) -> Result<()> {
        match &self.inner {
            TransformInner::F32(t) => t
                .transform(src, dst)
                .map_err(|e| Error::Transform(format!("{:?}", e))),
            _ => Err(Error::Transform("Wrong bit depth for transform".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_channels() {
        assert_eq!(Layout::Rgb.channels(), 3);
        assert_eq!(Layout::Rgba.channels(), 4);
        assert_eq!(Layout::Gray.channels(), 1);
        assert_eq!(Layout::GrayAlpha.channels(), 2);
    }

    #[test]
    fn test_transform_identity() {
        let profile = ColorProfile::new_srgb();
        let transform = Transform::new_8bit(
            &profile,
            Layout::Rgb,
            &profile,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .unwrap();

        let src = [255u8, 128, 64];
        let mut dst = [0u8; 3];
        transform.transform(&src, &mut dst).unwrap();

        // Identity transform should preserve values (with possible minor rounding)
        assert!((dst[0] as i32 - 255).abs() <= 1);
        assert!((dst[1] as i32 - 128).abs() <= 1);
        assert!((dst[2] as i32 - 64).abs() <= 1);
    }

    #[test]
    fn test_transform_srgb_to_p3() {
        let srgb = ColorProfile::new_srgb();
        let p3 = ColorProfile::new_display_p3();

        let transform = Transform::new_8bit(
            &srgb,
            Layout::Rgb,
            &p3,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .unwrap();

        // Transform pure red
        let src = [255u8, 0, 0];
        let mut dst = [0u8; 3];
        transform.transform(&src, &mut dst).unwrap();

        // sRGB red should become less saturated in P3
        assert!(dst[0] < 255);
        assert!(dst[1] > 0); // Will have some green
        assert!(dst[2] > 0); // Will have some blue
    }
}
