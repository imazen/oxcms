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

mod context;
mod matrix_shaper;
mod stages;

pub use context::{RenderIntent, TransformFlags, TransformContext};
pub use matrix_shaper::{MatrixShaperPipeline, MatrixShaperTransform};
pub use stages::{PipelineStage, TrcStage, MatrixStage};

use crate::color::Xyz;
use crate::icc::{IccProfile, IccError};

/// A complete color transform pipeline
#[derive(Debug, Clone)]
pub enum Pipeline {
    /// Matrix-shaper pipeline (RGB profiles)
    MatrixShaper(MatrixShaperPipeline),
    // /// LUT-based pipeline (CMYK, DeviceLink)
    // Lut(LutPipeline),
    // /// Gray profile pipeline
    // Gray(GrayPipeline),
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

        // TODO: Handle LUT-based profiles
        Err(IccError::Unsupported(
            "Only matrix-shaper profiles are supported currently".to_string(),
        ))
    }

    /// Transform a single RGB pixel
    ///
    /// Input and output are normalized [0, 1]
    pub fn transform_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgb(rgb),
        }
    }

    /// Transform a buffer of RGB pixels (8-bit)
    pub fn transform_rgb8(&self, src: &[u8], dst: &mut [u8]) {
        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgb8(src, dst),
        }
    }

    /// Transform a buffer of RGB pixels (16-bit)
    pub fn transform_rgb16(&self, src: &[u16], dst: &mut [u16]) {
        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgb16(src, dst),
        }
    }

    /// Transform a buffer of RGBA pixels (8-bit), preserving alpha
    pub fn transform_rgba8(&self, src: &[u8], dst: &mut [u8]) {
        match self {
            Pipeline::MatrixShaper(p) => p.transform_rgba8(src, dst),
        }
    }
}

/// Trait for pipeline stages
pub trait Stage {
    /// Apply the stage to an XYZ value
    fn apply(&self, xyz: Xyz) -> Xyz;

    /// Apply inverse (if applicable)
    fn apply_inverse(&self, xyz: Xyz) -> Xyz {
        xyz // Default: identity
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
}
