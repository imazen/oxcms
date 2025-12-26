//! Reference implementation wrappers
//!
//! Provides unified interfaces to call reference CMS implementations.

use crate::parity::ReferenceCms;

/// Transform using moxcms reference implementation
pub fn transform_moxcms(
    src_profile_data: &[u8],
    dst_profile_data: &[u8],
    src_pixels: &[u8],
) -> Result<Vec<u8>, String> {
    use moxcms::{ColorProfile, Layout, TransformOptions};

    let src_profile = ColorProfile::new_from_slice(src_profile_data)
        .map_err(|e| format!("moxcms src profile: {:?}", e))?;

    let dst_profile = ColorProfile::new_from_slice(dst_profile_data)
        .map_err(|e| format!("moxcms dst profile: {:?}", e))?;

    let transform = src_profile
        .create_transform_8bit(
            Layout::Rgb,
            &dst_profile,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .map_err(|e| format!("moxcms transform: {:?}", e))?;

    let mut dst_pixels = vec![0u8; src_pixels.len()];
    transform
        .transform(src_pixels, &mut dst_pixels)
        .map_err(|e| format!("moxcms execute: {:?}", e))?;

    Ok(dst_pixels)
}

/// Transform using moxcms with built-in sRGB
pub fn transform_moxcms_srgb(src_pixels: &[u8]) -> Result<Vec<u8>, String> {
    use moxcms::{ColorProfile, Layout, TransformOptions};

    let src_profile = ColorProfile::new_srgb();
    let dst_profile = ColorProfile::new_srgb();

    let transform = src_profile
        .create_transform_8bit(
            Layout::Rgb,
            &dst_profile,
            Layout::Rgb,
            TransformOptions::default(),
        )
        .map_err(|e| format!("moxcms transform: {:?}", e))?;

    let mut dst_pixels = vec![0u8; src_pixels.len()];
    transform
        .transform(src_pixels, &mut dst_pixels)
        .map_err(|e| format!("moxcms execute: {:?}", e))?;

    Ok(dst_pixels)
}

/// Transform using lcms2 reference implementation
pub fn transform_lcms2(
    src_profile_data: &[u8],
    dst_profile_data: &[u8],
    src_pixels: &[u8],
) -> Result<Vec<u8>, String> {
    use lcms2::{Intent, PixelFormat, Profile, Transform};

    let src_profile = Profile::new_icc(src_profile_data)
        .map_err(|e| format!("lcms2 src profile: {}", e))?;

    let dst_profile = Profile::new_icc(dst_profile_data)
        .map_err(|e| format!("lcms2 dst profile: {}", e))?;

    let transform = Transform::new(
        &src_profile,
        PixelFormat::RGB_8,
        &dst_profile,
        PixelFormat::RGB_8,
        Intent::Perceptual,
    )
    .map_err(|e| format!("lcms2 transform: {}", e))?;

    let mut dst_pixels = vec![0u8; src_pixels.len()];
    transform.transform_pixels(src_pixels, &mut dst_pixels);

    Ok(dst_pixels)
}

/// Transform using lcms2 with built-in sRGB
pub fn transform_lcms2_srgb(src_pixels: &[u8]) -> Result<Vec<u8>, String> {
    use lcms2::{Intent, PixelFormat, Profile, Transform};

    let src_profile = Profile::new_srgb();
    let dst_profile = Profile::new_srgb();

    let transform = Transform::new(
        &src_profile,
        PixelFormat::RGB_8,
        &dst_profile,
        PixelFormat::RGB_8,
        Intent::Perceptual,
    )
    .map_err(|e| format!("lcms2 transform: {}", e))?;

    let mut dst_pixels = vec![0u8; src_pixels.len()];
    transform.transform_pixels(src_pixels, &mut dst_pixels);

    Ok(dst_pixels)
}

/// Compare outputs from two reference implementations
pub fn compare_references(
    ref_a: ReferenceCms,
    ref_b: ReferenceCms,
    src_profile_data: &[u8],
    dst_profile_data: &[u8],
    src_pixels: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let output_a = match ref_a {
        ReferenceCms::Moxcms => transform_moxcms(src_profile_data, dst_profile_data, src_pixels)?,
        ReferenceCms::Lcms2 => transform_lcms2(src_profile_data, dst_profile_data, src_pixels)?,
        _ => return Err(format!("Unsupported reference: {:?}", ref_a)),
    };

    let output_b = match ref_b {
        ReferenceCms::Moxcms => transform_moxcms(src_profile_data, dst_profile_data, src_pixels)?,
        ReferenceCms::Lcms2 => transform_lcms2(src_profile_data, dst_profile_data, src_pixels)?,
        _ => return Err(format!("Unsupported reference: {:?}", ref_b)),
    };

    Ok((output_a, output_b))
}
