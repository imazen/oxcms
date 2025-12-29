//! CIELAB (L*a*b*) Color Space
//!
//! L*a*b* is a perceptually uniform color space where equal distances
//! correspond to roughly equal perceived color differences.
//!
//! - L*: Lightness (0 = black, 100 = white)
//! - a*: Green-red axis (negative = green, positive = red)
//! - b*: Blue-yellow axis (negative = blue, positive = yellow)

use crate::color::{D50, WhitePoint, Xyz};

/// CIELAB color coordinates
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Lab {
    /// Lightness (0 to 100)
    pub l: f64,
    /// Green-red axis (typically -128 to 127)
    pub a: f64,
    /// Blue-yellow axis (typically -128 to 127)
    pub b: f64,
}

impl Lab {
    /// Create a new Lab color
    #[inline]
    pub const fn new(l: f64, a: f64, b: f64) -> Self {
        Self { l, a, b }
    }

    /// Create Lab from an array
    #[inline]
    pub const fn from_array(arr: [f64; 3]) -> Self {
        Self {
            l: arr[0],
            a: arr[1],
            b: arr[2],
        }
    }

    /// Convert to array
    #[inline]
    pub const fn to_array(&self) -> [f64; 3] {
        [self.l, self.a, self.b]
    }

    /// Convert from XYZ with D50 white point (ICC PCS)
    pub fn from_xyz(xyz: Xyz) -> Self {
        Self::from_xyz_with_white(xyz, &D50)
    }

    /// Convert from XYZ with a specific white point
    pub fn from_xyz_with_white(xyz: Xyz, white: &WhitePoint) -> Self {
        let xr = xyz.x / white.xyz.x;
        let yr = xyz.y / white.xyz.y;
        let zr = xyz.z / white.xyz.z;

        let fx = lab_f(xr);
        let fy = lab_f(yr);
        let fz = lab_f(zr);

        Self {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }

    /// Convert to XYZ with D50 white point (ICC PCS)
    pub fn to_xyz(&self) -> Xyz {
        self.to_xyz_with_white(&D50)
    }

    /// Convert to XYZ with a specific white point
    pub fn to_xyz_with_white(&self, white: &WhitePoint) -> Xyz {
        let fy = (self.l + 16.0) / 116.0;
        let fx = self.a / 500.0 + fy;
        let fz = fy - self.b / 200.0;

        let xr = lab_f_inv(fx);
        let yr = lab_f_inv(fy);
        let zr = lab_f_inv(fz);

        Xyz::new(xr * white.xyz.x, yr * white.xyz.y, zr * white.xyz.z)
    }

    /// Get chroma (colorfulness)
    #[inline]
    pub fn chroma(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    /// Get hue angle in radians
    #[inline]
    pub fn hue(&self) -> f64 {
        self.b.atan2(self.a)
    }

    /// Get hue angle in degrees (0-360)
    #[inline]
    pub fn hue_degrees(&self) -> f64 {
        let h = self.hue().to_degrees();
        if h < 0.0 { h + 360.0 } else { h }
    }

    /// Check if approximately equal to another Lab color
    #[inline]
    pub fn approx_eq(&self, other: &Self, epsilon: f64) -> bool {
        (self.l - other.l).abs() < epsilon
            && (self.a - other.a).abs() < epsilon
            && (self.b - other.b).abs() < epsilon
    }
}

/// Lab forward function: f(t) for XYZ → Lab conversion
#[inline]
fn lab_f(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    const DELTA_CUBED: f64 = DELTA * DELTA * DELTA;

    if t > DELTA_CUBED {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

/// Lab inverse function: f⁻¹(t) for Lab → XYZ conversion
#[inline]
fn lab_f_inv(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;

    if t > DELTA {
        t * t * t
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

/// Calculate CIEDE2000 color difference
///
/// This is the industry-standard color difference formula.
/// A difference of 1.0 is approximately the just-noticeable difference.
pub fn delta_e_2000(lab1: Lab, lab2: Lab) -> f64 {
    // CIEDE2000 implementation
    // Reference: http://www.brucelindbloom.com/index.html?Eqn_DeltaE_CIE2000.html

    let l1 = lab1.l;
    let a1 = lab1.a;
    let b1 = lab1.b;
    let l2 = lab2.l;
    let a2 = lab2.a;
    let b2 = lab2.b;

    // Step 1: Calculate C and h
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;

    let c_bar_7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar_7 / (c_bar_7 + 25.0_f64.powi(7))).sqrt());

    let a1_prime = a1 * (1.0 + g);
    let a2_prime = a2 * (1.0 + g);

    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();

    let h1_prime = if a1_prime == 0.0 && b1 == 0.0 {
        0.0
    } else {
        let h = b1.atan2(a1_prime).to_degrees();
        if h < 0.0 { h + 360.0 } else { h }
    };

    let h2_prime = if a2_prime == 0.0 && b2 == 0.0 {
        0.0
    } else {
        let h = b2.atan2(a2_prime).to_degrees();
        if h < 0.0 { h + 360.0 } else { h }
    };

    // Step 2: Calculate deltas
    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;

    let delta_h_prime = if c1_prime * c2_prime == 0.0 {
        0.0
    } else {
        let diff = h2_prime - h1_prime;
        if diff.abs() <= 180.0 {
            diff
        } else if diff > 180.0 {
            diff - 360.0
        } else {
            diff + 360.0
        }
    };

    let delta_big_h_prime =
        2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime.to_radians() / 2.0).sin();

    // Step 3: Calculate CIEDE2000
    let l_bar_prime = (l1 + l2) / 2.0;
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;

    let h_bar_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * (h_bar_prime - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar_prime).to_radians().cos()
        + 0.32 * (3.0 * h_bar_prime + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar_prime - 63.0).to_radians().cos();

    let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let c_bar_prime_7 = c_bar_prime.powi(7);
    let r_c = 2.0 * (c_bar_prime_7 / (c_bar_prime_7 + 25.0_f64.powi(7))).sqrt();
    let s_l =
        1.0 + (0.015 * (l_bar_prime - 50.0).powi(2)) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;
    let r_t = -(2.0 * delta_theta).to_radians().sin() * r_c;

    // Weighting factors (commonly 1:1:1)
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    let term1 = delta_l_prime / (k_l * s_l);
    let term2 = delta_c_prime / (k_c * s_c);
    let term3 = delta_big_h_prime / (k_h * s_h);

    (term1 * term1 + term2 * term2 + term3 * term3 + r_t * term2 * term3).sqrt()
}

impl From<[f64; 3]> for Lab {
    fn from(arr: [f64; 3]) -> Self {
        Self::from_array(arr)
    }
}

impl From<Lab> for [f64; 3] {
    fn from(lab: Lab) -> Self {
        lab.to_array()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_white_is_100() {
        // D50 white should give L=100, a=0, b=0
        let white = Xyz::new(D50.xyz.x, D50.xyz.y, D50.xyz.z);
        let lab = Lab::from_xyz(white);
        assert!((lab.l - 100.0).abs() < EPSILON);
        assert!(lab.a.abs() < EPSILON);
        assert!(lab.b.abs() < EPSILON);
    }

    #[test]
    fn test_black_is_0() {
        let black = Xyz::new(0.0, 0.0, 0.0);
        let lab = Lab::from_xyz(black);
        assert!(lab.l.abs() < EPSILON);
    }

    #[test]
    fn test_roundtrip() {
        let original = Lab::new(50.0, 25.0, -30.0);
        let xyz = original.to_xyz();
        let roundtrip = Lab::from_xyz(xyz);

        assert!(
            original.approx_eq(&roundtrip, 1e-9),
            "Roundtrip failed: {:?} vs {:?}",
            original,
            roundtrip
        );
    }

    #[test]
    fn test_delta_e_identical() {
        let lab = Lab::new(50.0, 25.0, -30.0);
        let de = delta_e_2000(lab, lab);
        assert!(de.abs() < EPSILON, "Identical colors should have ΔE=0");
    }

    #[test]
    fn test_delta_e_perceptible() {
        // Very different colors should have high ΔE
        let red = Lab::new(50.0, 50.0, 0.0);
        let green = Lab::new(50.0, -50.0, 0.0);
        let de = delta_e_2000(red, green);
        assert!(de > 50.0, "Very different colors should have high ΔE");
    }

    #[test]
    fn test_chroma() {
        let lab = Lab::new(50.0, 3.0, 4.0);
        assert!((lab.chroma() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_hue() {
        let lab = Lab::new(50.0, 1.0, 0.0);
        assert!(lab.hue().abs() < EPSILON); // 0 degrees

        let lab = Lab::new(50.0, 0.0, 1.0);
        assert!((lab.hue() - std::f64::consts::FRAC_PI_2).abs() < EPSILON); // 90 degrees
    }
}
