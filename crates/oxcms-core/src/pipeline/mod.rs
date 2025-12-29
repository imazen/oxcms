//! Native Color Transform Pipeline
//!
//! This module provides native color transforms without depending on moxcms.
//! It supports:
//! - Matrix-shaper profiles (most RGB profiles)
//! - LUT-based profiles (CMYK, DeviceLink)
//! - Gray profiles
//!
//! # Pipeline Architecture
//!
//! A color transform consists of:
//! 1. Input stage: device space → PCS (Profile Connection Space)
//! 2. Chromatic adaptation: adapt between white points if needed
//! 3. Output stage: PCS → device space
//!
//! For matrix-shaper profiles:
//! - Input: TRC decode → matrix → PCS
//! - Output: PCS → inverse matrix → TRC encode
//!
//! For LUT profiles:
//! - Use A2B/B2A lookup tables directly

mod bpc;
mod context;
mod lut;
mod matrix_shaper;
mod stages;

pub use bpc::{BpcParams, detect_black_point};
pub use context::{RenderIntent, TransformContext, TransformFlags};
pub use lut::{ClutData, LutCurve, LutPipeline};
pub use matrix_shaper::{MatrixShaperPipeline, MatrixShaperTransform};
pub use stages::{MatrixStage, PipelineStage, TrcStage};

use crate::icc::{IccError, IccProfile, IccRenderingIntent, TagSignature};

/// A complete color transform pipeline
#[derive(Debug, Clone)]
pub enum Pipeline {
    /// Matrix-shaper pipeline (RGB profiles)
    MatrixShaper(MatrixShaperPipeline),
    /// LUT-based pipeline (CMYK, DeviceLink)
    Lut(LutPipeline),
    /// Chained LUT pipeline (source A2B + destination B2A)
    ChainedLut {
        /// Source profile LUT (device → PCS)
        source: LutPipeline,
        /// Destination profile LUT (PCS → device)
        destination: LutPipeline,
    },
}

impl Pipeline {
    /// Create a pipeline from two ICC profiles
    pub fn from_profiles(
        src: &IccProfile,
        dst: &IccProfile,
        ctx: &TransformContext,
    ) -> Result<Self, IccError> {
        // Check if both profiles are matrix-shaper
        if src.is_matrix_shaper() && dst.is_matrix_shaper() {
            let pipeline = MatrixShaperPipeline::from_profiles(src, dst, ctx)?;
            return Ok(Pipeline::MatrixShaper(pipeline));
        }

        // Handle LUT-based profiles (CMYK, DeviceLink, etc.)
        Self::from_lut_profiles(src, dst, ctx)
    }

    /// Create a pipeline from LUT-based profiles
    fn from_lut_profiles(
        src: &IccProfile,
        dst: &IccProfile,
        ctx: &TransformContext,
    ) -> Result<Self, IccError> {
        let intent = match ctx.intent {
            RenderIntent::Perceptual => IccRenderingIntent::Perceptual,
            RenderIntent::RelativeColorimetric => IccRenderingIntent::RelativeColorimetric,
            RenderIntent::Saturation => IccRenderingIntent::Saturation,
            RenderIntent::AbsoluteColorimetric => IccRenderingIntent::AbsoluteColorimetric,
        };

        // Get source A2B LUT (device → PCS)
        let src_tag = src
            .a2b_for_intent(intent)
            .ok_or(IccError::MissingTag(TagSignature::A2B0.0))?;
        let source_lut = LutPipeline::from_tag_data(src_tag)?;

        // Get destination B2A LUT (PCS → device)
        let dst_tag = dst
            .b2a_for_intent(intent)
            .ok_or(IccError::MissingTag(TagSignature::B2A0.0))?;
        let destination_lut = LutPipeline::from_tag_data(dst_tag)?;

        Ok(Pipeline::ChainedLut {
            source: source_lut,
            destination: destination_lut,
        })
    }

    /// Create a pipeline for CMYK → RGB transform
    pub fn cmyk_to_rgb(
        cmyk_profile: &IccProfile,
        rgb_profile: &IccProfile,
    ) -> Result<Self, IccError> {
        if !cmyk_profile.is_cmyk() {
            return Err(IccError::Unsupported(
                "Source profile is not CMYK".to_string(),
            ));
        }

        let ctx = TransformContext::default();
        Self::from_lut_profiles(cmyk_profile, rgb_profile, &ctx)
    }

    /// Create a pipeline for RGB → CMYK transform
    pub fn rgb_to_cmyk(
        rgb_profile: &IccProfile,
        cmyk_profile: &IccProfile,
    ) -> Result<Self, IccError> {
        if !cmyk_profile.is_cmyk() {
            return Err(IccError::Unsupported(
                "Destination profile is not CMYK".to_string(),
            ));
        }

        let ctx = TransformContext::default();
        Self::from_lut_profiles(rgb_profile, cmyk_profile, &ctx)
    }

    /// Transform a single RGB pixel
    ///
    /// Input and output are normalized [0, 1]
    pub fn transform_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgb(rgb),
            Pipeline::Lut(p) => p.transform_rgb(rgb),
            Pipeline::ChainedLut {
                source,
                destination,
            } => {
                // Source: device → PCS (3 channels)
                let pcs = source.transform_rgb(rgb);
                // Destination: PCS → device (3 channels for RGB)
                destination.transform_rgb(pcs)
            }
        }
    }

    /// Transform CMYK to RGB
    ///
    /// Input: CMYK [0, 1], Output: RGB [0, 1]
    pub fn transform_cmyk_to_rgb(&self, cmyk: [f64; 4]) -> [f64; 3] {
        match self {
            Pipeline::ChainedLut {
                source,
                destination,
            } => {
                // Source: CMYK → PCS (via A2B LUT)
                let pcs = source.transform(&cmyk);
                // PCS is typically Lab or XYZ (3 channels)
                let pcs_rgb = [
                    pcs.first().copied().unwrap_or(0.0),
                    pcs.get(1).copied().unwrap_or(0.0),
                    pcs.get(2).copied().unwrap_or(0.0),
                ];
                // Destination: PCS → RGB (via B2A LUT)
                destination.transform_rgb(pcs_rgb)
            }
            Pipeline::Lut(p) => {
                // Direct CMYK → RGB LUT
                let result = p.transform(&cmyk);
                [
                    result.first().copied().unwrap_or(0.0),
                    result.get(1).copied().unwrap_or(0.0),
                    result.get(2).copied().unwrap_or(0.0),
                ]
            }
            Pipeline::MatrixShaper(_) => {
                // Matrix-shaper doesn't support CMYK
                [0.0, 0.0, 0.0]
            }
        }
    }

    /// Transform RGB to CMYK
    ///
    /// Input: RGB [0, 1], Output: CMYK [0, 1]
    pub fn transform_rgb_to_cmyk(&self, rgb: [f64; 3]) -> [f64; 4] {
        match self {
            Pipeline::ChainedLut {
                source,
                destination,
            } => {
                // Source: RGB → PCS (via A2B LUT, 3ch → 3ch)
                let pcs = source.transform_rgb(rgb);
                // Destination: PCS → CMYK (via B2A LUT, 3ch → 4ch)
                // Use generic transform since B2A takes 3ch PCS and outputs 4ch CMYK
                let result = destination.transform(&pcs);
                [
                    result.first().copied().unwrap_or(0.0),
                    result.get(1).copied().unwrap_or(0.0),
                    result.get(2).copied().unwrap_or(0.0),
                    result.get(3).copied().unwrap_or(0.0),
                ]
            }
            Pipeline::Lut(p) => {
                // Direct RGB → CMYK LUT
                let result = p.transform(&rgb);
                [
                    result.first().copied().unwrap_or(0.0),
                    result.get(1).copied().unwrap_or(0.0),
                    result.get(2).copied().unwrap_or(0.0),
                    result.get(3).copied().unwrap_or(0.0),
                ]
            }
            Pipeline::MatrixShaper(_) => {
                // Matrix-shaper doesn't support CMYK
                [0.0, 0.0, 0.0, 0.0]
            }
        }
    }

    /// Transform a buffer of RGB pixels (8-bit)
    pub fn transform_rgb8(&self, src: &[u8], dst: &mut [u8]) {
        assert!(src.len() % 3 == 0);
        assert!(dst.len() >= src.len());

        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgb8(src, dst),
            Pipeline::Lut(_) | Pipeline::ChainedLut { .. } => {
                for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(3)) {
                    let rgb = [
                        src_chunk[0] as f64 / 255.0,
                        src_chunk[1] as f64 / 255.0,
                        src_chunk[2] as f64 / 255.0,
                    ];
                    let result = self.transform_rgb(rgb);
                    dst_chunk[0] = (result[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    dst_chunk[1] = (result[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    dst_chunk[2] = (result[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
            }
        }
    }

    /// Transform a buffer of RGB pixels (16-bit)
    pub fn transform_rgb16(&self, src: &[u16], dst: &mut [u16]) {
        assert!(src.len() % 3 == 0);
        assert!(dst.len() >= src.len());

        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgb16(src, dst),
            Pipeline::Lut(_) | Pipeline::ChainedLut { .. } => {
                for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(3)) {
                    let rgb = [
                        src_chunk[0] as f64 / 65535.0,
                        src_chunk[1] as f64 / 65535.0,
                        src_chunk[2] as f64 / 65535.0,
                    ];
                    let result = self.transform_rgb(rgb);
                    dst_chunk[0] = (result[0].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
                    dst_chunk[1] = (result[1].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
                    dst_chunk[2] = (result[2].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
                }
            }
        }
    }

    /// Transform a buffer of RGBA pixels (8-bit), preserving alpha
    pub fn transform_rgba8(&self, src: &[u8], dst: &mut [u8]) {
        assert!(src.len() % 4 == 0);
        assert!(dst.len() >= src.len());

        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgba8(src, dst),
            Pipeline::Lut(_) | Pipeline::ChainedLut { .. } => {
                for (src_chunk, dst_chunk) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
                    let rgb = [
                        src_chunk[0] as f64 / 255.0,
                        src_chunk[1] as f64 / 255.0,
                        src_chunk[2] as f64 / 255.0,
                    ];
                    let result = self.transform_rgb(rgb);
                    dst_chunk[0] = (result[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    dst_chunk[1] = (result[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    dst_chunk[2] = (result[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    dst_chunk[3] = src_chunk[3]; // Preserve alpha
                }
            }
        }
    }

    /// Transform CMYK buffer (8-bit) to RGB buffer (8-bit)
    pub fn transform_cmyk8_to_rgb8(&self, src: &[u8], dst: &mut [u8]) {
        assert!(src.len() % 4 == 0);
        let pixel_count = src.len() / 4;
        assert!(dst.len() >= pixel_count * 3);

        for (src_chunk, dst_chunk) in src.chunks_exact(4).zip(dst.chunks_exact_mut(3)) {
            let cmyk = [
                src_chunk[0] as f64 / 255.0,
                src_chunk[1] as f64 / 255.0,
                src_chunk[2] as f64 / 255.0,
                src_chunk[3] as f64 / 255.0,
            ];
            let rgb = self.transform_cmyk_to_rgb(cmyk);
            dst_chunk[0] = (rgb[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            dst_chunk[1] = (rgb[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            dst_chunk[2] = (rgb[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    }

    /// Transform RGB buffer (8-bit) to CMYK buffer (8-bit)
    pub fn transform_rgb8_to_cmyk8(&self, src: &[u8], dst: &mut [u8]) {
        assert!(src.len() % 3 == 0);
        let pixel_count = src.len() / 3;
        assert!(dst.len() >= pixel_count * 4);

        for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(4)) {
            let rgb = [
                src_chunk[0] as f64 / 255.0,
                src_chunk[1] as f64 / 255.0,
                src_chunk[2] as f64 / 255.0,
            ];
            let cmyk = self.transform_rgb_to_cmyk(rgb);
            dst_chunk[0] = (cmyk[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            dst_chunk[1] = (cmyk[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            dst_chunk[2] = (cmyk[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            dst_chunk[3] = (cmyk[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    }

    /// Transform CMYK buffer (16-bit) to RGB buffer (16-bit)
    pub fn transform_cmyk16_to_rgb16(&self, src: &[u16], dst: &mut [u16]) {
        assert!(src.len() % 4 == 0);
        let pixel_count = src.len() / 4;
        assert!(dst.len() >= pixel_count * 3);

        for (src_chunk, dst_chunk) in src.chunks_exact(4).zip(dst.chunks_exact_mut(3)) {
            let cmyk = [
                src_chunk[0] as f64 / 65535.0,
                src_chunk[1] as f64 / 65535.0,
                src_chunk[2] as f64 / 65535.0,
                src_chunk[3] as f64 / 65535.0,
            ];
            let rgb = self.transform_cmyk_to_rgb(cmyk);
            dst_chunk[0] = (rgb[0].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
            dst_chunk[1] = (rgb[1].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
            dst_chunk[2] = (rgb[2].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
        }
    }

    /// Transform RGB buffer (16-bit) to CMYK buffer (16-bit)
    pub fn transform_rgb16_to_cmyk16(&self, src: &[u16], dst: &mut [u16]) {
        assert!(src.len() % 3 == 0);
        let pixel_count = src.len() / 3;
        assert!(dst.len() >= pixel_count * 4);

        for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(4)) {
            let rgb = [
                src_chunk[0] as f64 / 65535.0,
                src_chunk[1] as f64 / 65535.0,
                src_chunk[2] as f64 / 65535.0,
            ];
            let cmyk = self.transform_rgb_to_cmyk(rgb);
            dst_chunk[0] = (cmyk[0].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
            dst_chunk[1] = (cmyk[1].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
            dst_chunk[2] = (cmyk[2].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
            dst_chunk[3] = (cmyk[3].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_exists() {
        // Placeholder test
        assert!(true);
    }

    #[test]
    fn test_chained_lut_pipeline_structure() {
        // Test that ChainedLut variant can be created with LutPipeline components
        let source = LutPipeline::identity(4, 3); // CMYK → PCS
        let destination = LutPipeline::identity(3, 4); // PCS → CMYK

        let pipeline = Pipeline::ChainedLut {
            source,
            destination,
        };

        // Verify the pipeline matches the expected variant
        match &pipeline {
            Pipeline::ChainedLut {
                source,
                destination,
            } => {
                assert_eq!(source.input_channels, 4);
                assert_eq!(source.output_channels, 3);
                assert_eq!(destination.input_channels, 3);
                assert_eq!(destination.output_channels, 4);
            }
            _ => panic!("Expected ChainedLut variant"),
        }
    }

    #[test]
    fn test_cmyk_transform_api() {
        // Test the CMYK transform API with identity pipelines
        let source = LutPipeline::identity(4, 3); // CMYK → PCS (3ch)
        let destination = LutPipeline::identity(3, 3); // PCS → RGB

        let pipeline = Pipeline::ChainedLut {
            source,
            destination,
        };

        // Test CMYK to RGB transform (with identity, first 3 channels pass through)
        let cmyk = [0.5, 0.3, 0.2, 0.1];
        let rgb = pipeline.transform_cmyk_to_rgb(cmyk);

        // With no CLUT and identity curves, the first 3 channels pass through
        assert!((rgb[0] - 0.5).abs() < 1e-10, "R: {}", rgb[0]);
        assert!((rgb[1] - 0.3).abs() < 1e-10, "G: {}", rgb[1]);
        assert!((rgb[2] - 0.2).abs() < 1e-10, "B: {}", rgb[2]);
    }

    #[test]
    fn test_rgb_to_cmyk_transform_api() {
        // Test RGB → CMYK with identity pipeline
        let source = LutPipeline::identity(3, 3); // RGB → PCS
        let destination = LutPipeline::identity(3, 4); // PCS → CMYK

        let pipeline = Pipeline::ChainedLut {
            source,
            destination,
        };

        let rgb = [0.5, 0.3, 0.2];
        let cmyk = pipeline.transform_rgb_to_cmyk(rgb);

        // With no CLUT and identity curves, the first 3 channels pass through
        // K channel will be 0 (4th output)
        assert!((cmyk[0] - 0.5).abs() < 1e-10, "C: {}", cmyk[0]);
        assert!((cmyk[1] - 0.3).abs() < 1e-10, "M: {}", cmyk[1]);
        assert!((cmyk[2] - 0.2).abs() < 1e-10, "Y: {}", cmyk[2]);
    }

    #[test]
    fn test_buffer_transforms_cmyk8_to_rgb8() {
        // Test 8-bit buffer transforms
        let source = LutPipeline::identity(4, 3); // CMYK → PCS
        let destination = LutPipeline::identity(3, 3); // PCS → RGB

        let pipeline = Pipeline::ChainedLut {
            source,
            destination,
        };

        // CMYK8 to RGB8
        let cmyk8 = [128u8, 64, 32, 16];
        let mut rgb8 = [0u8; 3];
        pipeline.transform_cmyk8_to_rgb8(&cmyk8, &mut rgb8);

        // Check output is reasonable (identity passes first 3 channels)
        assert_eq!(rgb8[0], 128);
        assert_eq!(rgb8[1], 64);
        assert_eq!(rgb8[2], 32);
    }

    #[test]
    fn test_buffer_transforms_rgb8_to_cmyk8() {
        // Test RGB8 to CMYK8
        let source = LutPipeline::identity(3, 3); // RGB → PCS
        let destination = LutPipeline::identity(3, 4); // PCS → CMYK

        let pipeline = Pipeline::ChainedLut {
            source,
            destination,
        };

        let rgb8 = [200u8, 100, 50];
        let mut cmyk8 = [0u8; 4];
        pipeline.transform_rgb8_to_cmyk8(&rgb8, &mut cmyk8);

        // Check output (first 3 channels pass through, K is 0)
        assert_eq!(cmyk8[0], 200);
        assert_eq!(cmyk8[1], 100);
        assert_eq!(cmyk8[2], 50);
        assert_eq!(cmyk8[3], 0); // K channel from identity is 0
    }

    #[test]
    fn test_16bit_cmyk_transforms() {
        // Test 16-bit transforms
        let source = LutPipeline::identity(4, 3);
        let destination = LutPipeline::identity(3, 3);

        let pipeline = Pipeline::ChainedLut {
            source,
            destination,
        };

        let cmyk16 = [32768u16, 16384, 8192, 4096];
        let mut rgb16 = [0u16; 3];
        pipeline.transform_cmyk16_to_rgb16(&cmyk16, &mut rgb16);

        // First 3 channels pass through
        assert_eq!(rgb16[0], 32768);
        assert_eq!(rgb16[1], 16384);
        assert_eq!(rgb16[2], 8192);
    }
}
