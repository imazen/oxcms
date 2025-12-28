//! CIE Standard Illuminant White Points
//!
//! White points define the color of "white" for a given illuminant.
//! These are specified as CIE XYZ coordinates where Y=1.0.
//!
//! Values are from CIE standards and ICC.1:2022.

use crate::color::Xyz;

/// A white point definition
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhitePoint {
    /// Name of the illuminant
    pub name: &'static str,
    /// CIE XYZ coordinates (Y normalized to 1.0)
    pub xyz: Xyz,
}

impl WhitePoint {
    /// Create a new white point
    pub const fn new(name: &'static str, x: f64, y: f64, z: f64) -> Self {
        Self {
            name,
            xyz: Xyz::new(x, y, z),
        }
    }

    /// Get the chromaticity coordinates (x, y)
    pub fn chromaticity(&self) -> (f64, f64) {
        let sum = self.xyz.x + self.xyz.y + self.xyz.z;
        if sum > 0.0 {
            (self.xyz.x / sum, self.xyz.y / sum)
        } else {
            (0.0, 0.0)
        }
    }
}

// ============================================================================
// Standard CIE Illuminants
// ============================================================================

/// CIE Standard Illuminant D50 (Horizon Light)
///
/// Correlated Color Temperature: ~5003K
/// Used as the Profile Connection Space (PCS) white point in ICC profiles.
pub const D50: WhitePoint = WhitePoint::new("D50", 0.9642, 1.0, 0.8251);

/// CIE Standard Illuminant D55 (Mid-morning/Mid-afternoon Daylight)
///
/// Correlated Color Temperature: ~5500K
pub const D55: WhitePoint = WhitePoint::new("D55", 0.9568, 1.0, 0.9214);

/// CIE Standard Illuminant D60
///
/// Correlated Color Temperature: ~6000K
/// Used in ACES color spaces.
pub const D60: WhitePoint = WhitePoint::new("D60", 0.9523, 1.0, 1.0084);

/// CIE Standard Illuminant D65 (Noon Daylight)
///
/// Correlated Color Temperature: ~6504K
/// Standard white point for sRGB, Adobe RGB, and most display color spaces.
pub const D65: WhitePoint = WhitePoint::new("D65", 0.9505, 1.0, 1.0890);

/// CIE Standard Illuminant D75 (North Sky Daylight)
///
/// Correlated Color Temperature: ~7500K
pub const D75: WhitePoint = WhitePoint::new("D75", 0.9497, 1.0, 1.2264);

/// DCI-P3 theatrical white point
///
/// Slightly greenish compared to D65.
pub const DCI_P3: WhitePoint = WhitePoint::new("DCI-P3", 0.8940, 1.0, 0.9544);

/// CIE Standard Illuminant A (Incandescent)
///
/// Correlated Color Temperature: ~2856K
pub const A: WhitePoint = WhitePoint::new("A", 1.0985, 1.0, 0.3558);

/// CIE Standard Illuminant E (Equal Energy)
///
/// Theoretical illuminant with equal power at all wavelengths.
pub const E: WhitePoint = WhitePoint::new("E", 1.0, 1.0, 1.0);

/// CIE Standard Illuminant F2 (Cool White Fluorescent)
pub const F2: WhitePoint = WhitePoint::new("F2", 0.9918, 1.0, 0.6739);

/// CIE Standard Illuminant F7 (Broadband Daylight Fluorescent)
pub const F7: WhitePoint = WhitePoint::new("F7", 0.9504, 1.0, 1.0872);

/// CIE Standard Illuminant F11 (Narrow Band White Fluorescent)
pub const F11: WhitePoint = WhitePoint::new("F11", 1.0092, 1.0, 0.6428);

// ============================================================================
// Utility functions
// ============================================================================

/// Check if two white points are approximately equal
pub fn white_points_equal(a: &WhitePoint, b: &WhitePoint, epsilon: f64) -> bool {
    (a.xyz.x - b.xyz.x).abs() < epsilon
        && (a.xyz.y - b.xyz.y).abs() < epsilon
        && (a.xyz.z - b.xyz.z).abs() < epsilon
}

/// Get a standard white point by name
pub fn from_name(name: &str) -> Option<WhitePoint> {
    match name.to_uppercase().as_str() {
        "D50" => Some(D50),
        "D55" => Some(D55),
        "D60" => Some(D60),
        "D65" => Some(D65),
        "D75" => Some(D75),
        "DCI-P3" | "DCI" => Some(DCI_P3),
        "A" => Some(A),
        "E" => Some(E),
        "F2" => Some(F2),
        "F7" => Some(F7),
        "F11" => Some(F11),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_d50_values() {
        // ICC.1:2022 specifies D50 as the PCS illuminant
        // X = 0.9642, Y = 1.0, Z = 0.8249 (we use 0.8251 which is also common)
        assert!((D50.xyz.x - 0.9642).abs() < 0.001);
        assert!((D50.xyz.y - 1.0).abs() < 0.001);
        assert!((D50.xyz.z - 0.8251).abs() < 0.001);
    }

    #[test]
    fn test_d65_values() {
        // sRGB specification D65 values
        assert!((D65.xyz.x - 0.9505).abs() < 0.001);
        assert!((D65.xyz.y - 1.0).abs() < 0.001);
        assert!((D65.xyz.z - 1.0890).abs() < 0.001);
    }

    #[test]
    fn test_chromaticity() {
        // D65 chromaticity should be approximately (0.3127, 0.3290)
        let (x, y) = D65.chromaticity();
        assert!((x - 0.3127).abs() < 0.001);
        assert!((y - 0.3290).abs() < 0.001);
    }

    #[test]
    fn test_from_name() {
        assert!(from_name("D50").is_some());
        assert!(from_name("d65").is_some());
        assert!(from_name("DCI-P3").is_some());
        assert!(from_name("unknown").is_none());
    }

    #[test]
    fn test_white_points_equal() {
        assert!(white_points_equal(&D65, &D65, 0.001));
        assert!(!white_points_equal(&D65, &D50, 0.001));
    }
}
