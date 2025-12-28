//! Chromatic Adaptation Transforms
//!
//! Chromatic adaptation transforms convert colors from one white point to another.
//! The most commonly used method is Bradford, which is the ICC default.
//!
//! References:
//! - ICC.1:2022 Annex E
//! - Lindbloom: http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html

use crate::color::{WhitePoint, Xyz};
use crate::math::Matrix3x3;

/// Chromatic adaptation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChromaticAdaptationMethod {
    /// Bradford adaptation (ICC default, recommended)
    #[default]
    Bradford,
    /// Von Kries adaptation
    VonKries,
    /// XYZ Scaling (simple but less accurate)
    XyzScaling,
    /// No adaptation (identity)
    None,
}

// ============================================================================
// Adaptation matrices (to/from cone response space)
// ============================================================================

/// Bradford matrix: XYZ → LMS (cone response)
const BRADFORD_XYZ_TO_LMS: Matrix3x3 = Matrix3x3::new([
    [0.8951000, 0.2664000, -0.1614000],
    [-0.7502000, 1.7135000, 0.0367000],
    [0.0389000, -0.0685000, 1.0296000],
]);

/// Bradford matrix: LMS → XYZ (inverse)
const BRADFORD_LMS_TO_XYZ: Matrix3x3 = Matrix3x3::new([
    [0.9869929, -0.1470543, 0.1599627],
    [0.4323053, 0.5183603, 0.0492912],
    [-0.0085287, 0.0400428, 0.9684867],
]);

/// Von Kries matrix: XYZ → LMS
const VON_KRIES_XYZ_TO_LMS: Matrix3x3 = Matrix3x3::new([
    [0.4002400, 0.7076000, -0.0808100],
    [-0.2263000, 1.1653200, 0.0457000],
    [0.0000000, 0.0000000, 0.9182200],
]);

/// Von Kries matrix: LMS → XYZ
const VON_KRIES_LMS_TO_XYZ: Matrix3x3 = Matrix3x3::new([
    [1.8599364, -1.1293816, 0.2198974],
    [0.3611914, 0.6388125, -0.0000064],
    [0.0000000, 0.0000000, 1.0890636],
]);

/// Get the XYZ to LMS matrix for a given method
fn xyz_to_lms_matrix(method: ChromaticAdaptationMethod) -> Matrix3x3 {
    match method {
        ChromaticAdaptationMethod::Bradford => BRADFORD_XYZ_TO_LMS,
        ChromaticAdaptationMethod::VonKries => VON_KRIES_XYZ_TO_LMS,
        ChromaticAdaptationMethod::XyzScaling | ChromaticAdaptationMethod::None => {
            Matrix3x3::identity()
        }
    }
}

/// Get the LMS to XYZ matrix for a given method
fn lms_to_xyz_matrix(method: ChromaticAdaptationMethod) -> Matrix3x3 {
    match method {
        ChromaticAdaptationMethod::Bradford => BRADFORD_LMS_TO_XYZ,
        ChromaticAdaptationMethod::VonKries => VON_KRIES_LMS_TO_XYZ,
        ChromaticAdaptationMethod::XyzScaling | ChromaticAdaptationMethod::None => {
            Matrix3x3::identity()
        }
    }
}

/// Compute the chromatic adaptation matrix for converting from one white point to another
///
/// The returned matrix M can be used as: XYZ_dest = M × XYZ_src
///
/// # Arguments
/// * `src_white` - Source white point
/// * `dst_white` - Destination white point
/// * `method` - Adaptation method to use
pub fn adaptation_matrix(
    src_white: &WhitePoint,
    dst_white: &WhitePoint,
    method: ChromaticAdaptationMethod,
) -> Matrix3x3 {
    if method == ChromaticAdaptationMethod::None {
        return Matrix3x3::identity();
    }

    // Get the transformation matrices
    let m_a = xyz_to_lms_matrix(method);
    let m_a_inv = lms_to_xyz_matrix(method);

    // Convert white points to LMS
    let src_lms = m_a.multiply_vec(src_white.xyz.to_array());
    let dst_lms = m_a.multiply_vec(dst_white.xyz.to_array());

    // Build the diagonal scaling matrix
    let scale = if method == ChromaticAdaptationMethod::XyzScaling {
        // For XYZ scaling, use XYZ directly
        Matrix3x3::diagonal(
            dst_white.xyz.x / src_white.xyz.x,
            dst_white.xyz.y / src_white.xyz.y,
            dst_white.xyz.z / src_white.xyz.z,
        )
    } else {
        // For Bradford/Von Kries, scale in LMS space
        Matrix3x3::diagonal(
            if src_lms[0].abs() > 1e-10 {
                dst_lms[0] / src_lms[0]
            } else {
                1.0
            },
            if src_lms[1].abs() > 1e-10 {
                dst_lms[1] / src_lms[1]
            } else {
                1.0
            },
            if src_lms[2].abs() > 1e-10 {
                dst_lms[2] / src_lms[2]
            } else {
                1.0
            },
        )
    };

    if method == ChromaticAdaptationMethod::XyzScaling {
        scale
    } else {
        // M = M_A^-1 × Scale × M_A
        m_a_inv.multiply(&scale.multiply(&m_a))
    }
}

/// Compute the Bradford adaptation matrix (convenience function)
///
/// This is the most commonly used method and is the ICC default.
#[inline]
pub fn bradford_matrix(src_white: &WhitePoint, dst_white: &WhitePoint) -> Matrix3x3 {
    adaptation_matrix(src_white, dst_white, ChromaticAdaptationMethod::Bradford)
}

/// Adapt an XYZ color from one white point to another
///
/// # Arguments
/// * `xyz` - Color to adapt
/// * `src_white` - Source white point
/// * `dst_white` - Destination white point
/// * `method` - Adaptation method to use
#[inline]
pub fn adapt_xyz(
    xyz: Xyz,
    src_white: &WhitePoint,
    dst_white: &WhitePoint,
    method: ChromaticAdaptationMethod,
) -> Xyz {
    let matrix = adaptation_matrix(src_white, dst_white, method);
    Xyz::from_array(matrix.multiply_vec(xyz.to_array()))
}

/// Adapt an XYZ color using Bradford adaptation (convenience function)
#[inline]
pub fn adapt_xyz_bradford(xyz: Xyz, src_white: &WhitePoint, dst_white: &WhitePoint) -> Xyz {
    adapt_xyz(xyz, src_white, dst_white, ChromaticAdaptationMethod::Bradford)
}

/// Pre-computed D65 → D50 Bradford matrix
///
/// This is the most common adaptation in ICC workflows.
pub const D65_TO_D50_BRADFORD: Matrix3x3 = Matrix3x3::new([
    [1.0478112, 0.0228866, -0.0501270],
    [0.0295424, 0.9904844, -0.0170491],
    [-0.0092345, 0.0150436, 0.7521316],
]);

/// Pre-computed D50 → D65 Bradford matrix
pub const D50_TO_D65_BRADFORD: Matrix3x3 = Matrix3x3::new([
    [0.9555766, -0.0230393, 0.0631636],
    [-0.0282895, 1.0099416, 0.0210077],
    [0.0122982, -0.0204830, 1.3299098],
]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::white_point::{D50, D65};

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_identity_adaptation() {
        // Adapting from D65 to D65 should be identity
        let matrix = bradford_matrix(&D65, &D65);
        assert!(matrix.is_identity(EPSILON));
    }

    #[test]
    fn test_d65_to_d50() {
        // Test against pre-computed matrix
        // Note: Pre-computed values may differ slightly due to rounding in constants
        let computed = bradford_matrix(&D65, &D50);
        assert!(
            computed.approx_eq(&D65_TO_D50_BRADFORD, 1e-2),
            "D65→D50 matrix mismatch: computed={:?} expected={:?}",
            computed,
            D65_TO_D50_BRADFORD
        );
    }

    #[test]
    fn test_d50_to_d65() {
        // Test against pre-computed matrix
        let computed = bradford_matrix(&D50, &D65);
        assert!(
            computed.approx_eq(&D50_TO_D65_BRADFORD, 1e-2),
            "D50→D65 matrix mismatch: computed={:?} expected={:?}",
            computed,
            D50_TO_D65_BRADFORD
        );
    }

    #[test]
    fn test_adaptation_roundtrip() {
        // D65 → D50 → D65 should be identity
        let m1 = bradford_matrix(&D65, &D50);
        let m2 = bradford_matrix(&D50, &D65);
        let roundtrip = m1.multiply(&m2);
        assert!(roundtrip.is_identity(1e-5), "Roundtrip not identity");
    }

    #[test]
    fn test_white_point_adaptation() {
        // D65 white should map to D50 white
        let d65_white = D65.xyz;
        let adapted = adapt_xyz_bradford(d65_white, &D65, &D50);

        // Should be close to D50 white point
        assert!(
            adapted.approx_eq(&D50.xyz, 1e-4),
            "D65 white → D50: {:?} vs {:?}",
            adapted,
            D50.xyz
        );
    }

    #[test]
    fn test_xyz_scaling() {
        let matrix =
            adaptation_matrix(&D65, &D50, ChromaticAdaptationMethod::XyzScaling);

        // XYZ scaling is a diagonal matrix
        assert!(matrix.m[0][1].abs() < EPSILON);
        assert!(matrix.m[0][2].abs() < EPSILON);
        assert!(matrix.m[1][0].abs() < EPSILON);
        assert!(matrix.m[1][2].abs() < EPSILON);
        assert!(matrix.m[2][0].abs() < EPSILON);
        assert!(matrix.m[2][1].abs() < EPSILON);
    }

    #[test]
    fn test_color_adaptation() {
        // A known color in D65
        let color_d65 = Xyz::new(0.5, 0.5, 0.5);

        // Adapt to D50
        let color_d50 = adapt_xyz_bradford(color_d65, &D65, &D50);

        // Adapt back to D65
        let roundtrip = adapt_xyz_bradford(color_d50, &D50, &D65);

        // Tolerance accounts for floating-point precision in matrix operations
        assert!(
            color_d65.approx_eq(&roundtrip, 1e-7),
            "Color roundtrip failed: {:?} vs {:?}",
            color_d65,
            roundtrip
        );
    }
}
