//! Matrix-Shaper Transform Pipeline
//!
//! Matrix-shaper profiles are the most common ICC profile type (sRGB, Display P3, etc.).
//! They consist of:
//! 1. Input TRCs (one per channel) - decode to linear
//! 2. 3x3 colorant matrix - convert to/from XYZ
//! 3. Output TRCs (one per channel) - encode from linear
//!
//! # Transform Pipeline
//!
//! Source RGB → TRC decode → Matrix to XYZ → [Chromatic adaptation] → Matrix from XYZ → TRC encode → Dest RGB

use crate::color::{white_point::D50, WhitePoint};
use crate::icc::{IccError, IccProfile};
use crate::math::{adaptation_matrix, Matrix3x3};

use super::context::TransformContext;
use super::stages::{TrcCurve, TrcStage};

/// A matrix-shaper transform pipeline
#[derive(Debug, Clone)]
pub struct MatrixShaperPipeline {
    /// Input TRCs (decode)
    input_trc: TrcStage,
    /// Input matrix (device to PCS)
    input_matrix: Matrix3x3,
    /// Chromatic adaptation (if white points differ)
    adaptation: Option<Matrix3x3>,
    /// Output matrix (PCS to device)
    output_matrix: Matrix3x3,
    /// Output TRCs (encode)
    output_trc: TrcStage,
    /// Clamp output
    clamp: bool,
}

impl MatrixShaperPipeline {
    /// Create a pipeline from two ICC profiles
    pub fn from_profiles(
        src: &IccProfile,
        dst: &IccProfile,
        ctx: &TransformContext,
    ) -> Result<Self, IccError> {
        // Get source colorant matrix (RGB → XYZ)
        let src_matrix = Self::extract_colorant_matrix(src)?;

        // Get destination colorant matrix (RGB → XYZ)
        let dst_matrix = Self::extract_colorant_matrix(dst)?;

        // Invert destination matrix (XYZ → RGB)
        let dst_matrix_inv = dst_matrix.inverse().ok_or_else(|| {
            IccError::CorruptedData("Destination matrix is singular".to_string())
        })?;

        // Get TRCs
        let src_trc = TrcStage::from_curves(
            src.red_trc(),
            src.green_trc(),
            src.blue_trc(),
        );

        let dst_trc = TrcStage::from_curves(
            dst.red_trc(),
            dst.green_trc(),
            dst.blue_trc(),
        );

        // Check if chromatic adaptation is needed
        let src_white = src.media_white_point().unwrap_or(D50.xyz);
        let dst_white = dst.media_white_point().unwrap_or(D50.xyz);

        let adaptation = if !src_white.approx_eq(&dst_white, 1e-4) {
            // Need chromatic adaptation
            let src_wp = WhitePoint::from_xyz(src_white);
            let dst_wp = WhitePoint::from_xyz(dst_white);
            Some(adaptation_matrix(&src_wp, &dst_wp, ctx.adaptation_method))
        } else {
            None
        };

        Ok(Self {
            input_trc: src_trc,
            input_matrix: src_matrix,
            adaptation,
            output_matrix: dst_matrix_inv,
            output_trc: dst_trc,
            clamp: ctx.flags.clamp_output,
        })
    }

    /// Extract the colorant matrix from a profile
    fn extract_colorant_matrix(profile: &IccProfile) -> Result<Matrix3x3, IccError> {
        let red = profile.red_colorant().ok_or_else(|| {
            IccError::MissingTag(u32::from_be_bytes(*b"rXYZ"))
        })?;
        let green = profile.green_colorant().ok_or_else(|| {
            IccError::MissingTag(u32::from_be_bytes(*b"gXYZ"))
        })?;
        let blue = profile.blue_colorant().ok_or_else(|| {
            IccError::MissingTag(u32::from_be_bytes(*b"bXYZ"))
        })?;

        // Build matrix with colorants as columns
        Ok(Matrix3x3::new([
            [red.x, green.x, blue.x],
            [red.y, green.y, blue.y],
            [red.z, green.z, blue.z],
        ]))
    }

    /// Transform a single RGB pixel (normalized [0, 1])
    pub fn transform_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        // 1. Apply input TRC (decode to linear)
        let linear = self.input_trc.apply(rgb);

        // 2. Apply input matrix (device → XYZ)
        let mut xyz = self.input_matrix.multiply_vec(linear);

        // 3. Apply chromatic adaptation if needed
        if let Some(ref adapt) = self.adaptation {
            xyz = adapt.multiply_vec(xyz);
        }

        // 4. Apply output matrix (XYZ → device)
        let linear_out = self.output_matrix.multiply_vec(xyz);

        // 5. Apply output TRC (encode from linear)
        let mut result = self.output_trc.apply_inverse(linear_out);

        // 6. Clamp if requested
        if self.clamp {
            result = [
                result[0].clamp(0.0, 1.0),
                result[1].clamp(0.0, 1.0),
                result[2].clamp(0.0, 1.0),
            ];
        }

        result
    }

    /// Transform a buffer of 8-bit RGB pixels
    pub fn transform_rgb8(&self, src: &[u8], dst: &mut [u8]) {
        assert!(src.len() % 3 == 0);
        assert!(dst.len() >= src.len());

        for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(3)) {
            let rgb = [
                src_chunk[0] as f64 / 255.0,
                src_chunk[1] as f64 / 255.0,
                src_chunk[2] as f64 / 255.0,
            ];

            let result = self.transform_rgb(rgb);

            dst_chunk[0] = (result[0] * 255.0 + 0.5) as u8;
            dst_chunk[1] = (result[1] * 255.0 + 0.5) as u8;
            dst_chunk[2] = (result[2] * 255.0 + 0.5) as u8;
        }
    }

    /// Transform a buffer of 16-bit RGB pixels
    pub fn transform_rgb16(&self, src: &[u16], dst: &mut [u16]) {
        assert!(src.len() % 3 == 0);
        assert!(dst.len() >= src.len());

        for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(3)) {
            let rgb = [
                src_chunk[0] as f64 / 65535.0,
                src_chunk[1] as f64 / 65535.0,
                src_chunk[2] as f64 / 65535.0,
            ];

            let result = self.transform_rgb(rgb);

            dst_chunk[0] = (result[0] * 65535.0 + 0.5) as u16;
            dst_chunk[1] = (result[1] * 65535.0 + 0.5) as u16;
            dst_chunk[2] = (result[2] * 65535.0 + 0.5) as u16;
        }
    }

    /// Transform a buffer of 8-bit RGBA pixels (alpha preserved)
    pub fn transform_rgba8(&self, src: &[u8], dst: &mut [u8]) {
        assert!(src.len() % 4 == 0);
        assert!(dst.len() >= src.len());

        for (src_chunk, dst_chunk) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
            let rgb = [
                src_chunk[0] as f64 / 255.0,
                src_chunk[1] as f64 / 255.0,
                src_chunk[2] as f64 / 255.0,
            ];

            let result = self.transform_rgb(rgb);

            dst_chunk[0] = (result[0] * 255.0 + 0.5) as u8;
            dst_chunk[1] = (result[1] * 255.0 + 0.5) as u8;
            dst_chunk[2] = (result[2] * 255.0 + 0.5) as u8;
            dst_chunk[3] = src_chunk[3]; // Preserve alpha
        }
    }
}

/// Combined matrix-shaper transform (optimized single matrix + TRCs)
///
/// For profiles with the same white point, we can combine the matrices
/// into a single 3x3 matrix for better performance.
#[derive(Debug, Clone)]
pub struct MatrixShaperTransform {
    /// Combined matrix (device A → device B)
    pub combined_matrix: Matrix3x3,
    /// Input TRCs
    pub input_trc: TrcStage,
    /// Output TRCs
    pub output_trc: TrcStage,
    /// Clamp output
    pub clamp: bool,
}

impl MatrixShaperTransform {
    /// Create from two profiles (optimized version)
    pub fn from_profiles(
        src: &IccProfile,
        dst: &IccProfile,
        ctx: &TransformContext,
    ) -> Result<Self, IccError> {
        let pipeline = MatrixShaperPipeline::from_profiles(src, dst, ctx)?;

        // Combine matrices: output_matrix × [adaptation] × input_matrix
        let combined = if let Some(adapt) = &pipeline.adaptation {
            pipeline
                .output_matrix
                .multiply(&adapt.multiply(&pipeline.input_matrix))
        } else {
            pipeline.output_matrix.multiply(&pipeline.input_matrix)
        };

        Ok(Self {
            combined_matrix: combined,
            input_trc: pipeline.input_trc,
            output_trc: pipeline.output_trc,
            clamp: pipeline.clamp,
        })
    }

    /// Transform a single RGB pixel
    pub fn transform_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        // 1. Apply input TRC
        let linear = self.input_trc.apply(rgb);

        // 2. Apply combined matrix
        let linear_out = self.combined_matrix.multiply_vec(linear);

        // 3. Apply output TRC
        let mut result = self.output_trc.apply_inverse(linear_out);

        // 4. Clamp if requested
        if self.clamp {
            result = [
                result[0].clamp(0.0, 1.0),
                result[1].clamp(0.0, 1.0),
                result[2].clamp(0.0, 1.0),
            ];
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::matrix::{SRGB_TO_XYZ, XYZ_TO_SRGB};

    #[test]
    fn test_srgb_identity_transform() {
        // Create an identity-like pipeline (same profile in and out)
        let pipeline = MatrixShaperPipeline {
            input_trc: TrcStage {
                red: TrcCurve::Gamma(2.2),
                green: TrcCurve::Gamma(2.2),
                blue: TrcCurve::Gamma(2.2),
            },
            input_matrix: SRGB_TO_XYZ,
            adaptation: None,
            output_matrix: XYZ_TO_SRGB,
            output_trc: TrcStage {
                red: TrcCurve::Gamma(2.2),
                green: TrcCurve::Gamma(2.2),
                blue: TrcCurve::Gamma(2.2),
            },
            clamp: true,
        };

        // White should stay white
        let white = [1.0, 1.0, 1.0];
        let result = pipeline.transform_rgb(white);
        assert!(
            (result[0] - 1.0).abs() < 0.001,
            "White R: {}",
            result[0]
        );
        assert!(
            (result[1] - 1.0).abs() < 0.001,
            "White G: {}",
            result[1]
        );
        assert!(
            (result[2] - 1.0).abs() < 0.001,
            "White B: {}",
            result[2]
        );

        // Black should stay black
        let black = [0.0, 0.0, 0.0];
        let result = pipeline.transform_rgb(black);
        assert!((result[0] - 0.0).abs() < 0.001, "Black R: {}", result[0]);
        assert!((result[1] - 0.0).abs() < 0.001, "Black G: {}", result[1]);
        assert!((result[2] - 0.0).abs() < 0.001, "Black B: {}", result[2]);

        // Mid-gray should stay approximately the same
        let gray = [0.5, 0.5, 0.5];
        let result = pipeline.transform_rgb(gray);
        assert!(
            (result[0] - 0.5).abs() < 0.01,
            "Gray R: {}",
            result[0]
        );
        assert!(
            (result[1] - 0.5).abs() < 0.01,
            "Gray G: {}",
            result[1]
        );
        assert!(
            (result[2] - 0.5).abs() < 0.01,
            "Gray B: {}",
            result[2]
        );
    }

    #[test]
    fn test_rgb8_transform() {
        let pipeline = MatrixShaperPipeline {
            input_trc: TrcStage {
                red: TrcCurve::Identity,
                green: TrcCurve::Identity,
                blue: TrcCurve::Identity,
            },
            input_matrix: Matrix3x3::identity(),
            adaptation: None,
            output_matrix: Matrix3x3::identity(),
            output_trc: TrcStage {
                red: TrcCurve::Identity,
                green: TrcCurve::Identity,
                blue: TrcCurve::Identity,
            },
            clamp: true,
        };

        let src = [255u8, 128, 64, 0, 255, 128];
        let mut dst = [0u8; 6];

        pipeline.transform_rgb8(&src, &mut dst);

        assert_eq!(dst[0], 255);
        assert_eq!(dst[1], 128);
        assert_eq!(dst[2], 64);
        assert_eq!(dst[3], 0);
        assert_eq!(dst[4], 255);
        assert_eq!(dst[5], 128);
    }

    #[test]
    fn test_rgba8_alpha_preserved() {
        let pipeline = MatrixShaperPipeline {
            input_trc: TrcStage {
                red: TrcCurve::Identity,
                green: TrcCurve::Identity,
                blue: TrcCurve::Identity,
            },
            input_matrix: Matrix3x3::identity(),
            adaptation: None,
            output_matrix: Matrix3x3::identity(),
            output_trc: TrcStage {
                red: TrcCurve::Identity,
                green: TrcCurve::Identity,
                blue: TrcCurve::Identity,
            },
            clamp: true,
        };

        let src = [255u8, 128, 64, 200]; // RGBA with alpha=200
        let mut dst = [0u8; 4];

        pipeline.transform_rgba8(&src, &mut dst);

        assert_eq!(dst[3], 200, "Alpha should be preserved");
    }

    #[test]
    fn test_combined_transform() {
        let transform = MatrixShaperTransform {
            combined_matrix: Matrix3x3::identity(),
            input_trc: TrcStage {
                red: TrcCurve::Identity,
                green: TrcCurve::Identity,
                blue: TrcCurve::Identity,
            },
            output_trc: TrcStage {
                red: TrcCurve::Identity,
                green: TrcCurve::Identity,
                blue: TrcCurve::Identity,
            },
            clamp: true,
        };

        let rgb = [0.5, 0.3, 0.7];
        let result = transform.transform_rgb(rgb);

        assert!((result[0] - 0.5).abs() < 1e-10);
        assert!((result[1] - 0.3).abs() < 1e-10);
        assert!((result[2] - 0.7).abs() < 1e-10);
    }
}
