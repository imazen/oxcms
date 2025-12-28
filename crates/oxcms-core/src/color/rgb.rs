//! RGB Color Primitives
//!
//! This module provides RGB color types and conversions.

use std::ops::{Add, Mul, Sub};

/// RGB color in floating-point (0.0-1.0 range)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rgb {
    /// Red component (0.0 to 1.0)
    pub r: f64,
    /// Green component (0.0 to 1.0)
    pub g: f64,
    /// Blue component (0.0 to 1.0)
    pub b: f64,
}

impl Rgb {
    /// Create a new RGB color
    #[inline]
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Create RGB from an array
    #[inline]
    pub const fn from_array(arr: [f64; 3]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
        }
    }

    /// Convert to array
    #[inline]
    pub const fn to_array(&self) -> [f64; 3] {
        [self.r, self.g, self.b]
    }

    /// Create from 8-bit values (0-255)
    #[inline]
    pub fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
        }
    }

    /// Convert to 8-bit values (0-255)
    #[inline]
    pub fn to_u8(&self) -> [u8; 3] {
        [
            (self.r * 255.0).round().clamp(0.0, 255.0) as u8,
            (self.g * 255.0).round().clamp(0.0, 255.0) as u8,
            (self.b * 255.0).round().clamp(0.0, 255.0) as u8,
        ]
    }

    /// Create from 16-bit values (0-65535)
    #[inline]
    pub fn from_u16(r: u16, g: u16, b: u16) -> Self {
        Self {
            r: r as f64 / 65535.0,
            g: g as f64 / 65535.0,
            b: b as f64 / 65535.0,
        }
    }

    /// Convert to 16-bit values (0-65535)
    #[inline]
    pub fn to_u16(&self) -> [u16; 3] {
        [
            (self.r * 65535.0).round().clamp(0.0, 65535.0) as u16,
            (self.g * 65535.0).round().clamp(0.0, 65535.0) as u16,
            (self.b * 65535.0).round().clamp(0.0, 65535.0) as u16,
        ]
    }

    /// Clamp all components to [0, 1]
    #[inline]
    pub fn clamp(&self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
        }
    }

    /// Check if all components are in [0, 1]
    #[inline]
    pub fn is_in_gamut(&self) -> bool {
        self.r >= 0.0
            && self.r <= 1.0
            && self.g >= 0.0
            && self.g <= 1.0
            && self.b >= 0.0
            && self.b <= 1.0
    }

    /// Calculate luminance using Rec. 709 coefficients
    #[inline]
    pub fn luminance(&self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Scale all components by a factor
    #[inline]
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
        }
    }

    /// Check if approximately equal to another RGB color
    #[inline]
    pub fn approx_eq(&self, other: &Self, epsilon: f64) -> bool {
        (self.r - other.r).abs() < epsilon
            && (self.g - other.g).abs() < epsilon
            && (self.b - other.b).abs() < epsilon
    }

    /// Black color
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0);

    /// White color
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);

    /// Red primary
    pub const RED: Self = Self::new(1.0, 0.0, 0.0);

    /// Green primary
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0);

    /// Blue primary
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0);
}

impl From<[f64; 3]> for Rgb {
    fn from(arr: [f64; 3]) -> Self {
        Self::from_array(arr)
    }
}

impl From<Rgb> for [f64; 3] {
    fn from(rgb: Rgb) -> Self {
        rgb.to_array()
    }
}

impl From<[u8; 3]> for Rgb {
    fn from(arr: [u8; 3]) -> Self {
        Self::from_u8(arr[0], arr[1], arr[2])
    }
}

impl Add for Rgb {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}

impl Sub for Rgb {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
        }
    }
}

impl Mul<f64> for Rgb {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        self.scale(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_u8_conversion() {
        let rgb = Rgb::from_u8(255, 128, 0);
        assert!((rgb.r - 1.0).abs() < EPSILON);
        assert!((rgb.g - 128.0 / 255.0).abs() < EPSILON);
        assert!((rgb.b - 0.0).abs() < EPSILON);

        let back = rgb.to_u8();
        assert_eq!(back, [255, 128, 0]);
    }

    #[test]
    fn test_u16_conversion() {
        let rgb = Rgb::from_u16(65535, 32768, 0);
        assert!((rgb.r - 1.0).abs() < 0.0001);
        assert!((rgb.g - 0.5).abs() < 0.001);
        assert!((rgb.b - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_clamp() {
        let rgb = Rgb::new(1.5, -0.5, 0.5);
        let clamped = rgb.clamp();
        assert_eq!(clamped.r, 1.0);
        assert_eq!(clamped.g, 0.0);
        assert_eq!(clamped.b, 0.5);
    }

    #[test]
    fn test_in_gamut() {
        assert!(Rgb::WHITE.is_in_gamut());
        assert!(Rgb::BLACK.is_in_gamut());
        assert!(!Rgb::new(1.5, 0.0, 0.0).is_in_gamut());
        assert!(!Rgb::new(0.0, -0.1, 0.0).is_in_gamut());
    }

    #[test]
    fn test_luminance() {
        assert!((Rgb::BLACK.luminance() - 0.0).abs() < EPSILON);
        assert!((Rgb::WHITE.luminance() - 1.0).abs() < EPSILON);

        // Green should have highest luminance contribution
        let g = Rgb::GREEN.luminance();
        let r = Rgb::RED.luminance();
        let b = Rgb::BLUE.luminance();
        assert!(g > r && g > b);
    }

    #[test]
    fn test_arithmetic() {
        let a = Rgb::new(0.5, 0.5, 0.5);
        let b = Rgb::new(0.1, 0.2, 0.3);

        let sum = a + b;
        assert!(sum.approx_eq(&Rgb::new(0.6, 0.7, 0.8), EPSILON));

        let scaled = a * 2.0;
        assert!(scaled.approx_eq(&Rgb::WHITE, EPSILON));
    }
}
