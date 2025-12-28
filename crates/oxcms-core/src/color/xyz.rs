//! CIE XYZ Color Space
//!
//! XYZ is the fundamental color space for color management.
//! All ICC profile transforms go through XYZ as the Profile Connection Space (PCS).

use std::ops::{Add, Mul, Sub};

/// CIE 1931 XYZ color coordinates
///
/// The XYZ color space is device-independent and encompasses all colors
/// visible to the human eye. Y represents luminance.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Xyz {
    /// X tristimulus value (mix of cone responses, roughly red)
    pub x: f64,
    /// Y tristimulus value (luminance)
    pub y: f64,
    /// Z tristimulus value (roughly blue)
    pub z: f64,
}

impl Xyz {
    /// Create a new XYZ color
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Create XYZ from an array
    #[inline]
    pub const fn from_array(arr: [f64; 3]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }

    /// Convert to array
    #[inline]
    pub const fn to_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Get the luminance (Y component)
    #[inline]
    pub const fn luminance(&self) -> f64 {
        self.y
    }

    /// Check if this is a valid color (all components non-negative)
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.x >= 0.0 && self.y >= 0.0 && self.z >= 0.0
    }

    /// Clamp to valid range (non-negative)
    #[inline]
    pub fn clamp_positive(&self) -> Self {
        Self {
            x: self.x.max(0.0),
            y: self.y.max(0.0),
            z: self.z.max(0.0),
        }
    }

    /// Scale all components by a factor
    #[inline]
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    /// Normalize so Y = 1.0
    #[inline]
    pub fn normalize(&self) -> Self {
        if self.y > 0.0 {
            self.scale(1.0 / self.y)
        } else {
            *self
        }
    }

    /// Convert to xyY chromaticity coordinates
    ///
    /// Returns (x, y, Y) where x and y are chromaticity and Y is luminance.
    #[inline]
    pub fn to_xyy(&self) -> (f64, f64, f64) {
        let sum = self.x + self.y + self.z;
        if sum > 0.0 {
            (self.x / sum, self.y / sum, self.y)
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    /// Create XYZ from xyY chromaticity coordinates
    #[inline]
    pub fn from_xyy(x: f64, y: f64, big_y: f64) -> Self {
        if y > 0.0 {
            Self {
                x: (x * big_y) / y,
                y: big_y,
                z: ((1.0 - x - y) * big_y) / y,
            }
        } else {
            Self::new(0.0, 0.0, 0.0)
        }
    }

    /// Check if approximately equal to another XYZ color
    #[inline]
    pub fn approx_eq(&self, other: &Self, epsilon: f64) -> bool {
        (self.x - other.x).abs() < epsilon
            && (self.y - other.y).abs() < epsilon
            && (self.z - other.z).abs() < epsilon
    }
}

impl From<[f64; 3]> for Xyz {
    fn from(arr: [f64; 3]) -> Self {
        Self::from_array(arr)
    }
}

impl From<Xyz> for [f64; 3] {
    fn from(xyz: Xyz) -> Self {
        xyz.to_array()
    }
}

impl Add for Xyz {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Xyz {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f64> for Xyz {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        self.scale(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let xyz = Xyz::new(0.5, 0.6, 0.7);
        assert_eq!(xyz.x, 0.5);
        assert_eq!(xyz.y, 0.6);
        assert_eq!(xyz.z, 0.7);
    }

    #[test]
    fn test_array_conversion() {
        let arr = [0.1, 0.2, 0.3];
        let xyz = Xyz::from_array(arr);
        assert_eq!(xyz.to_array(), arr);

        let xyz2: Xyz = arr.into();
        assert_eq!(xyz, xyz2);
    }

    #[test]
    fn test_xyy_roundtrip() {
        let original = Xyz::new(0.5, 0.6, 0.7);
        let (x, y, big_y) = original.to_xyy();
        let roundtrip = Xyz::from_xyy(x, y, big_y);

        assert!(original.approx_eq(&roundtrip, 1e-10));
    }

    #[test]
    fn test_normalize() {
        let xyz = Xyz::new(0.5, 0.25, 0.75);
        let normalized = xyz.normalize();
        assert!((normalized.y - 1.0).abs() < 1e-10);
        assert!((normalized.x - 2.0).abs() < 1e-10);
        assert!((normalized.z - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_arithmetic() {
        let a = Xyz::new(1.0, 2.0, 3.0);
        let b = Xyz::new(0.1, 0.2, 0.3);

        let sum = a + b;
        assert!(sum.approx_eq(&Xyz::new(1.1, 2.2, 3.3), 1e-10));

        let diff = a - b;
        assert!(diff.approx_eq(&Xyz::new(0.9, 1.8, 2.7), 1e-10));

        let scaled = a * 2.0;
        assert!(scaled.approx_eq(&Xyz::new(2.0, 4.0, 6.0), 1e-10));
    }
}
