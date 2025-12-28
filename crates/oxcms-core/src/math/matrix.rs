//! 3x3 Matrix operations for color space transforms
//!
//! These matrices are used for RGB↔XYZ conversions and chromatic adaptation.
//! All operations use f64 for precision, matching lcms2's internal precision.

use std::ops::{Index, IndexMut, Mul};

/// A 3x3 matrix for color space transformations
///
/// Stored in row-major order: m[row][col]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3 {
    /// Matrix elements in row-major order
    pub m: [[f64; 3]; 3],
}

impl Matrix3x3 {
    /// Create a new matrix from row-major elements
    #[inline]
    pub const fn new(m: [[f64; 3]; 3]) -> Self {
        Self { m }
    }

    /// Create an identity matrix
    #[inline]
    pub const fn identity() -> Self {
        Self {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Create a zero matrix
    #[inline]
    pub const fn zero() -> Self {
        Self {
            m: [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        }
    }

    /// Create a diagonal matrix from three values
    #[inline]
    pub const fn diagonal(d0: f64, d1: f64, d2: f64) -> Self {
        Self {
            m: [[d0, 0.0, 0.0], [0.0, d1, 0.0], [0.0, 0.0, d2]],
        }
    }

    /// Multiply this matrix by a 3-element vector
    ///
    /// Returns M × v
    #[inline]
    pub fn multiply_vec(&self, v: [f64; 3]) -> [f64; 3] {
        [
            self.m[0][0] * v[0] + self.m[0][1] * v[1] + self.m[0][2] * v[2],
            self.m[1][0] * v[0] + self.m[1][1] * v[1] + self.m[1][2] * v[2],
            self.m[2][0] * v[0] + self.m[2][1] * v[1] + self.m[2][2] * v[2],
        ]
    }

    /// Multiply this matrix by another matrix
    ///
    /// Returns self × other
    #[inline]
    pub fn multiply(&self, other: &Self) -> Self {
        let mut result = Self::zero();
        for i in 0..3 {
            for j in 0..3 {
                result.m[i][j] = self.m[i][0] * other.m[0][j]
                    + self.m[i][1] * other.m[1][j]
                    + self.m[i][2] * other.m[2][j];
            }
        }
        result
    }

    /// Transpose this matrix
    #[inline]
    pub fn transpose(&self) -> Self {
        Self {
            m: [
                [self.m[0][0], self.m[1][0], self.m[2][0]],
                [self.m[0][1], self.m[1][1], self.m[2][1]],
                [self.m[0][2], self.m[1][2], self.m[2][2]],
            ],
        }
    }

    /// Calculate the determinant
    #[inline]
    pub fn determinant(&self) -> f64 {
        let m = &self.m;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// Calculate the inverse of this matrix
    ///
    /// Returns None if the matrix is singular (determinant ≈ 0)
    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();

        // Check for singular matrix
        if det.abs() < 1e-14 {
            return None;
        }

        let inv_det = 1.0 / det;
        let m = &self.m;

        // Calculate adjugate matrix divided by determinant
        Some(Self {
            m: [
                [
                    (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
                    (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
                    (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
                ],
                [
                    (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
                    (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
                    (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
                ],
                [
                    (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
                    (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
                    (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
                ],
            ],
        })
    }

    /// Scale all elements by a scalar
    #[inline]
    pub fn scale(&self, s: f64) -> Self {
        Self {
            m: [
                [self.m[0][0] * s, self.m[0][1] * s, self.m[0][2] * s],
                [self.m[1][0] * s, self.m[1][1] * s, self.m[1][2] * s],
                [self.m[2][0] * s, self.m[2][1] * s, self.m[2][2] * s],
            ],
        }
    }

    /// Check if this matrix is approximately equal to another
    pub fn approx_eq(&self, other: &Self, epsilon: f64) -> bool {
        for i in 0..3 {
            for j in 0..3 {
                if (self.m[i][j] - other.m[i][j]).abs() > epsilon {
                    return false;
                }
            }
        }
        true
    }

    /// Check if this is approximately an identity matrix
    pub fn is_identity(&self, epsilon: f64) -> bool {
        self.approx_eq(&Self::identity(), epsilon)
    }
}

impl Default for Matrix3x3 {
    fn default() -> Self {
        Self::identity()
    }
}

impl Index<usize> for Matrix3x3 {
    type Output = [f64; 3];

    fn index(&self, row: usize) -> &Self::Output {
        &self.m[row]
    }
}

impl IndexMut<usize> for Matrix3x3 {
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.m[row]
    }
}

impl Mul for Matrix3x3 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.multiply(&rhs)
    }
}

impl Mul<[f64; 3]> for Matrix3x3 {
    type Output = [f64; 3];

    fn mul(self, rhs: [f64; 3]) -> Self::Output {
        self.multiply_vec(rhs)
    }
}

impl Mul<&[f64; 3]> for Matrix3x3 {
    type Output = [f64; 3];

    fn mul(self, rhs: &[f64; 3]) -> Self::Output {
        self.multiply_vec(*rhs)
    }
}

// ============================================================================
// Standard color space matrices (D65 white point, unless noted)
// ============================================================================

/// sRGB to XYZ matrix (D65 white point)
///
/// From IEC 61966-2-1:1999
pub const SRGB_TO_XYZ: Matrix3x3 = Matrix3x3::new([
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
]);

/// XYZ to sRGB matrix (D65 white point)
///
/// Inverse of SRGB_TO_XYZ
pub const XYZ_TO_SRGB: Matrix3x3 = Matrix3x3::new([
    [3.2404542, -1.5371385, -0.4985314],
    [-0.9692660, 1.8760108, 0.0415560],
    [0.0556434, -0.2040259, 1.0572252],
]);

/// Display P3 to XYZ matrix (D65 white point)
pub const DISPLAY_P3_TO_XYZ: Matrix3x3 = Matrix3x3::new([
    [0.4865709, 0.2656677, 0.1982173],
    [0.2289746, 0.6917385, 0.0792869],
    [0.0000000, 0.0451134, 1.0439444],
]);

/// XYZ to Display P3 matrix (D65 white point)
pub const XYZ_TO_DISPLAY_P3: Matrix3x3 = Matrix3x3::new([
    [2.4934969, -0.9313836, -0.4027108],
    [-0.8294890, 1.7626641, 0.0236247],
    [0.0358458, -0.0761724, 0.9568845],
]);

/// Adobe RGB (1998) to XYZ matrix (D65 white point)
pub const ADOBE_RGB_TO_XYZ: Matrix3x3 = Matrix3x3::new([
    [0.5767309, 0.1855540, 0.1881852],
    [0.2973769, 0.6273491, 0.0752741],
    [0.0270343, 0.0706872, 0.9911085],
]);

/// XYZ to Adobe RGB (1998) matrix (D65 white point)
pub const XYZ_TO_ADOBE_RGB: Matrix3x3 = Matrix3x3::new([
    [2.0413690, -0.5649464, -0.3446944],
    [-0.9692660, 1.8760108, 0.0415560],
    [0.0134474, -0.1183897, 1.0154096],
]);

/// BT.2020 to XYZ matrix (D65 white point)
pub const BT2020_TO_XYZ: Matrix3x3 = Matrix3x3::new([
    [0.6369580, 0.1446169, 0.1688810],
    [0.2627002, 0.6779981, 0.0593017],
    [0.0000000, 0.0280727, 1.0609851],
]);

/// XYZ to BT.2020 matrix (D65 white point)
pub const XYZ_TO_BT2020: Matrix3x3 = Matrix3x3::new([
    [1.7166512, -0.3556708, -0.2533663],
    [-0.6666844, 1.6164812, 0.0157685],
    [0.0176399, -0.0427706, 0.9421031],
]);

/// ProPhoto RGB to XYZ matrix (D50 white point)
pub const PROPHOTO_TO_XYZ_D50: Matrix3x3 = Matrix3x3::new([
    [0.7976749, 0.1351917, 0.0313534],
    [0.2880402, 0.7118741, 0.0000857],
    [0.0000000, 0.0000000, 0.8252100],
]);

/// XYZ to ProPhoto RGB matrix (D50 white point)
pub const XYZ_D50_TO_PROPHOTO: Matrix3x3 = Matrix3x3::new([
    [1.3459433, -0.2556075, -0.0511118],
    [-0.5445989, 1.5081673, 0.0205351],
    [0.0000000, 0.0000000, 1.2118128],
]);

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_identity() {
        let id = Matrix3x3::identity();
        let v = [1.0, 2.0, 3.0];
        let result = id.multiply_vec(v);
        assert!((result[0] - v[0]).abs() < EPSILON);
        assert!((result[1] - v[1]).abs() < EPSILON);
        assert!((result[2] - v[2]).abs() < EPSILON);
    }

    #[test]
    fn test_multiply_matrices() {
        let a = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        let id = Matrix3x3::identity();

        // A × I = A
        let result = a.multiply(&id);
        assert!(result.approx_eq(&a, EPSILON));

        // I × A = A
        let result = id.multiply(&a);
        assert!(result.approx_eq(&a, EPSILON));
    }

    #[test]
    fn test_transpose() {
        let a = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        let at = a.transpose();
        let expected = Matrix3x3::new([[1.0, 4.0, 7.0], [2.0, 5.0, 8.0], [3.0, 6.0, 9.0]]);
        assert!(at.approx_eq(&expected, EPSILON));

        // Transpose twice = original
        assert!(at.transpose().approx_eq(&a, EPSILON));
    }

    #[test]
    fn test_determinant() {
        let id = Matrix3x3::identity();
        assert!((id.determinant() - 1.0).abs() < EPSILON);

        let a = Matrix3x3::new([[1.0, 2.0, 3.0], [0.0, 1.0, 4.0], [5.0, 6.0, 0.0]]);
        assert!((a.determinant() - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_inverse() {
        let id = Matrix3x3::identity();
        let id_inv = id.inverse().unwrap();
        assert!(id_inv.approx_eq(&id, EPSILON));

        // A × A⁻¹ = I
        let a = Matrix3x3::new([[1.0, 2.0, 3.0], [0.0, 1.0, 4.0], [5.0, 6.0, 0.0]]);
        let a_inv = a.inverse().unwrap();
        let product = a.multiply(&a_inv);
        assert!(product.approx_eq(&id, 1e-9));
    }

    #[test]
    fn test_singular_matrix() {
        // Singular matrix (row 3 = row 1 + row 2)
        let singular = Matrix3x3::new([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [5.0, 7.0, 9.0]]);
        assert!(singular.inverse().is_none());
    }

    #[test]
    fn test_srgb_xyz_roundtrip() {
        // sRGB → XYZ → sRGB should be identity
        let roundtrip = SRGB_TO_XYZ.multiply(&XYZ_TO_SRGB);
        assert!(
            roundtrip.approx_eq(&Matrix3x3::identity(), 1e-6),
            "sRGB roundtrip failed"
        );
    }

    #[test]
    fn test_display_p3_xyz_roundtrip() {
        let roundtrip = DISPLAY_P3_TO_XYZ.multiply(&XYZ_TO_DISPLAY_P3);
        assert!(
            roundtrip.approx_eq(&Matrix3x3::identity(), 1e-6),
            "Display P3 roundtrip failed"
        );
    }

    #[test]
    fn test_known_srgb_to_xyz() {
        // sRGB white (1,1,1) should map to D65 white point
        let white = SRGB_TO_XYZ.multiply_vec([1.0, 1.0, 1.0]);
        // D65 white point: X=0.95047, Y=1.0, Z=1.08883
        assert!((white[0] - 0.95047).abs() < 0.001);
        assert!((white[1] - 1.0).abs() < 0.001);
        assert!((white[2] - 1.08883).abs() < 0.001);
    }

    #[test]
    fn test_operator_overloads() {
        let a = Matrix3x3::identity();
        let b = Matrix3x3::identity();
        let c = a * b;
        assert!(c.is_identity(EPSILON));

        let v = [1.0, 2.0, 3.0];
        let result = a * v;
        assert!((result[0] - 1.0).abs() < EPSILON);
        assert!((result[1] - 2.0).abs() < EPSILON);
        assert!((result[2] - 3.0).abs() < EPSILON);
    }
}
