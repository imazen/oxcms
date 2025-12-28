//! Black Point Compensation (BPC)
//!
//! Black point compensation adjusts the transform to map the source profile's
//! black point to the destination profile's black point. This prevents crushing
//! of dark tones when converting between color spaces with different black points.
//!
//! # Algorithm
//!
//! BPC performs a linear scaling in XYZ space:
//! - Calculate the offset from D50 white point for source and destination black points
//! - Scale XYZ values to map source black → destination black while preserving white
//!
//! # When to use BPC
//!
//! - Print workflows: CMYK profiles often have non-zero black points
//! - HDR to SDR conversion: Prevents black crushing
//! - Relative colorimetric intent: BPC is typically enabled by default

use crate::color::white_point::D50;
use crate::color::Xyz;

/// Black point compensation parameters
#[derive(Debug, Clone, Copy)]
pub struct BpcParams {
    /// Scale factors for XYZ
    pub scale: [f64; 3],
    /// Offset for XYZ
    pub offset: [f64; 3],
}

impl BpcParams {
    /// Calculate BPC parameters from source and destination black points
    ///
    /// # Arguments
    /// * `src_bp` - Source profile black point in XYZ
    /// * `dst_bp` - Destination profile black point in XYZ
    ///
    /// # Returns
    /// BPC parameters, or None if compensation is not needed
    pub fn calculate(src_bp: Xyz, dst_bp: Xyz) -> Option<Self> {
        let wp = D50.xyz;

        // Calculate deltas from white point
        let tx = src_bp.x - wp.x;
        let ty = src_bp.y - wp.y;
        let tz = src_bp.z - wp.z;

        // Avoid division by zero
        if tx.abs() < 1e-10 || ty.abs() < 1e-10 || tz.abs() < 1e-10 {
            return None;
        }

        // Calculate scale factors
        let scale = [
            (dst_bp.x - wp.x) / tx,
            (dst_bp.y - wp.y) / ty,
            (dst_bp.z - wp.z) / tz,
        ];

        // Calculate offsets
        let offset = [
            -wp.x * (dst_bp.x - src_bp.x) / tx,
            -wp.y * (dst_bp.y - src_bp.y) / ty,
            -wp.z * (dst_bp.z - src_bp.z) / tz,
        ];

        Some(Self { scale, offset })
    }

    /// Apply BPC to an XYZ value
    #[inline]
    pub fn apply(&self, xyz: [f64; 3]) -> [f64; 3] {
        [
            self.offset[0] + xyz[0] * self.scale[0],
            self.offset[1] + xyz[1] * self.scale[1],
            self.offset[2] + xyz[2] * self.scale[2],
        ]
    }

    /// Apply BPC to a buffer of XYZ values (in-place)
    pub fn apply_buffer(&self, buffer: &mut [f64]) {
        for chunk in buffer.chunks_exact_mut(3) {
            chunk[0] = self.offset[0] + chunk[0] * self.scale[0];
            chunk[1] = self.offset[1] + chunk[1] * self.scale[1];
            chunk[2] = self.offset[2] + chunk[2] * self.scale[2];
        }
    }
}

/// Detect black point for a profile
///
/// For RGB profiles, the black point is typically (0, 0, 0).
/// For CMYK profiles, we need to evaluate the A2B LUT at maximum ink coverage.
pub fn detect_black_point(
    profile: &crate::icc::IccProfile,
    explicit_bp: Option<Xyz>,
) -> Option<Xyz> {
    // Use explicit black point if available
    if let Some(bp) = explicit_bp {
        return Some(bp);
    }

    // Check profile's bkpt tag
    if let Some(bp) = profile.media_black_point() {
        return Some(bp);
    }

    // For RGB profiles, assume black is (0, 0, 0)
    if !profile.is_cmyk() && !profile.is_lut_based() {
        return Some(Xyz::new(0.0, 0.0, 0.0));
    }

    // For CMYK/LUT profiles without explicit black point, we would need to
    // evaluate the LUT at maximum K (or CMYK = 100,100,100,100)
    // This is a complex operation that requires the LUT pipeline
    // For now, return None and let the caller handle it
    None
}

/// Default black point for sRGB (pure black)
pub const SRGB_BLACK_POINT: Xyz = Xyz {
    x: 0.0,
    y: 0.0,
    z: 0.0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpc_identity() {
        // Same black points should result in identity transform
        let bp = Xyz::new(0.01, 0.01, 0.01);
        let params = BpcParams::calculate(bp, bp);

        // When src == dst, scale should be 1 and offset should be 0
        if let Some(p) = params {
            let input = [0.5, 0.5, 0.5];
            let output = p.apply(input);
            assert!((output[0] - input[0]).abs() < 0.01);
            assert!((output[1] - input[1]).abs() < 0.01);
            assert!((output[2] - input[2]).abs() < 0.01);
        }
    }

    #[test]
    fn test_bpc_preserves_white() {
        // BPC should preserve the white point
        let src_bp = Xyz::new(0.01, 0.01, 0.01);
        let dst_bp = Xyz::new(0.02, 0.02, 0.02);

        if let Some(params) = BpcParams::calculate(src_bp, dst_bp) {
            let wp = D50.xyz;
            let output = params.apply([wp.x, wp.y, wp.z]);

            // White point should be preserved (approximately)
            assert!(
                (output[0] - wp.x).abs() < 0.01,
                "White X: {} vs {}",
                output[0],
                wp.x
            );
            assert!(
                (output[1] - wp.y).abs() < 0.01,
                "White Y: {} vs {}",
                output[1],
                wp.y
            );
            assert!(
                (output[2] - wp.z).abs() < 0.01,
                "White Z: {} vs {}",
                output[2],
                wp.z
            );
        }
    }

    #[test]
    fn test_bpc_maps_black() {
        // BPC should map source black to destination black
        let src_bp = Xyz::new(0.01, 0.01, 0.01);
        let dst_bp = Xyz::new(0.02, 0.02, 0.02);

        if let Some(params) = BpcParams::calculate(src_bp, dst_bp) {
            let output = params.apply([src_bp.x, src_bp.y, src_bp.z]);

            // Source black should map to destination black
            assert!(
                (output[0] - dst_bp.x).abs() < 0.01,
                "Black X: {} vs {}",
                output[0],
                dst_bp.x
            );
            assert!(
                (output[1] - dst_bp.y).abs() < 0.01,
                "Black Y: {} vs {}",
                output[1],
                dst_bp.y
            );
            assert!(
                (output[2] - dst_bp.z).abs() < 0.01,
                "Black Z: {} vs {}",
                output[2],
                dst_bp.z
            );
        }
    }
}
