//! XYB Color Space
//!
//! XYB is a perceptually uniform color space designed for JPEG XL.
//! It's an LMS-based color model with cube root gamma (gamma 3).
//!
//! # Characteristics
//!
//! - **X channel**: L-M opponent (red-green difference)
//! - **Y channel**: Luminance-like ((L+M)/2)
//! - **B channel**: S-M (blue channel)
//!
//! # Usage
//!
//! ```
//! use oxcms_core::color::xyb::{Xyb, linear_rgb_to_xyb, xyb_to_linear_rgb};
//!
//! // Convert linear RGB to XYB
//! let xyb = linear_rgb_to_xyb(0.5, 0.5, 0.5);
//!
//! // Convert back
//! let (r, g, b) = xyb_to_linear_rgb(&xyb);
//! ```
//!
//! # Note
//!
//! XYB is **not** an ICC profile color space - it must be handled outside
//! the ICC pipeline. For JPEG XL decoding, convert XYB to linear RGB first,
//! then apply ICC transforms.

/// Bias added before cube root in forward transform
pub const BIAS: f64 = 0.003_793_073_255_275_449_3;

/// Cube root of BIAS, subtracted after cube root
pub const BIAS_CBRT: f64 = 0.155_954_200_549_248_63;

/// XYB color value
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xyb {
    /// X channel: L-M opponent (red-green)
    pub x: f64,
    /// Y channel: luminance-like (L+M)/2
    pub y: f64,
    /// B channel: S-M (blue)
    pub b: f64,
}

impl Xyb {
    /// Create a new XYB color
    pub const fn new(x: f64, y: f64, b: f64) -> Self {
        Self { x, y, b }
    }

    /// Black (all zeros)
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0);

    /// Convert to an array [x, y, b]
    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.b]
    }

    /// Create from an array [x, y, b]
    pub fn from_array(arr: [f64; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }
}

impl Default for Xyb {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Linear RGB color value (not gamma-encoded)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl LinearRgb {
    /// Create a new linear RGB color
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Create from sRGB values (0-255)
    pub fn from_srgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: srgb_to_linear(r as f64 / 255.0),
            g: srgb_to_linear(g as f64 / 255.0),
            b: srgb_to_linear(b as f64 / 255.0),
        }
    }

    /// Convert to sRGB values (0-255)
    pub fn to_srgb(self) -> (u8, u8, u8) {
        let r = (linear_to_srgb(self.r.clamp(0.0, 1.0)) * 255.0).round() as u8;
        let g = (linear_to_srgb(self.g.clamp(0.0, 1.0)) * 255.0).round() as u8;
        let b = (linear_to_srgb(self.b.clamp(0.0, 1.0)) * 255.0).round() as u8;
        (r, g, b)
    }

    /// Convert to an array [r, g, b]
    pub fn to_array(self) -> [f64; 3] {
        [self.r, self.g, self.b]
    }

    /// Create from an array [r, g, b]
    pub fn from_array(arr: [f64; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }
}

/// sRGB gamma to linear
#[inline]
pub fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear to sRGB gamma
#[inline]
pub fn linear_to_srgb(v: f64) -> f64 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Cube root function (handles negative values)
#[inline]
fn cbrt(x: f64) -> f64 {
    if x >= 0.0 {
        x.cbrt()
    } else {
        -(-x).cbrt()
    }
}

/// Convert linear RGB to XYB
///
/// Input: Linear RGB (not gamma-encoded), range [0, 1]
/// Output: XYB, approximate ranges X:[-0.05,0.05], Y:[0,0.85], B:[-0.45,0.45]
pub fn linear_rgb_to_xyb(r: f64, g: f64, b: f64) -> Xyb {
    // RGB to LMS matrix
    let l_linear = 0.3 * r + 0.622 * g + 0.078 * b;
    let m_linear = 0.23 * r + 0.692 * g + 0.078 * b;
    let s_linear =
        0.243_422_689_245_478_2 * r + 0.204_767_444_244_968_2 * g + 0.551_809_866_509_553_5 * b;

    // Apply bias and cube root
    let l_gamma = cbrt(l_linear + BIAS) - BIAS_CBRT;
    let m_gamma = cbrt(m_linear + BIAS) - BIAS_CBRT;
    let s_gamma = cbrt(s_linear + BIAS) - BIAS_CBRT;

    // LMS to XYB
    Xyb {
        x: (l_gamma - m_gamma) * 0.5,
        y: (l_gamma + m_gamma) * 0.5,
        b: s_gamma - m_gamma,
    }
}

/// Convert XYB to linear RGB
pub fn xyb_to_linear_rgb(xyb: &Xyb) -> (f64, f64, f64) {
    // XYB to LMS
    let l_gamma = xyb.x + xyb.y + BIAS_CBRT;
    let m_gamma = -xyb.x + xyb.y + BIAS_CBRT;
    let s_gamma = -xyb.x + xyb.y + xyb.b + BIAS_CBRT;

    // Apply cubic (inverse of cube root)
    let l_linear = l_gamma.powi(3) - BIAS;
    let m_linear = m_gamma.powi(3) - BIAS;
    let s_linear = s_gamma.powi(3) - BIAS;

    // LMS to RGB matrix (inverse of forward matrix)
    let r = 11.031566901960783 * l_linear - 9.866943921568629 * m_linear
        - 0.16462299647058826 * s_linear;
    let g = -3.254147380392157 * l_linear + 4.418770392156863 * m_linear
        - 0.16462299647058826 * s_linear;
    let b = -3.6588512862745097 * l_linear + 2.7129230470588235 * m_linear
        + 1.9459282392156863 * s_linear;

    (r, g, b)
}

/// Convert sRGB (0-255) to XYB
pub fn srgb_to_xyb(r: u8, g: u8, b: u8) -> Xyb {
    let linear = LinearRgb::from_srgb(r, g, b);
    linear_rgb_to_xyb(linear.r, linear.g, linear.b)
}

/// Convert XYB to sRGB (0-255)
pub fn xyb_to_srgb(xyb: &Xyb) -> (u8, u8, u8) {
    let (r, g, b) = xyb_to_linear_rgb(xyb);
    let linear = LinearRgb::new(r, g, b);
    linear.to_srgb()
}

/// Transform a buffer of XYB f32 values to linear RGB f32 values
///
/// Both buffers should be RGB triplets (length must be divisible by 3)
pub fn xyb_to_linear_rgb_buffer(xyb_data: &[f32], rgb_out: &mut [f32]) {
    assert_eq!(xyb_data.len(), rgb_out.len());
    assert_eq!(xyb_data.len() % 3, 0);

    for (xyb_chunk, rgb_chunk) in xyb_data.chunks_exact(3).zip(rgb_out.chunks_exact_mut(3)) {
        let xyb = Xyb::new(xyb_chunk[0] as f64, xyb_chunk[1] as f64, xyb_chunk[2] as f64);
        let (r, g, b) = xyb_to_linear_rgb(&xyb);
        rgb_chunk[0] = r as f32;
        rgb_chunk[1] = g as f32;
        rgb_chunk[2] = b as f32;
    }
}

/// Transform a buffer of linear RGB f32 values to XYB f32 values
///
/// Both buffers should be RGB triplets (length must be divisible by 3)
pub fn linear_rgb_to_xyb_buffer(rgb_data: &[f32], xyb_out: &mut [f32]) {
    assert_eq!(rgb_data.len(), xyb_out.len());
    assert_eq!(rgb_data.len() % 3, 0);

    for (rgb_chunk, xyb_chunk) in rgb_data.chunks_exact(3).zip(xyb_out.chunks_exact_mut(3)) {
        let xyb = linear_rgb_to_xyb(rgb_chunk[0] as f64, rgb_chunk[1] as f64, rgb_chunk[2] as f64);
        xyb_chunk[0] = xyb.x as f32;
        xyb_chunk[1] = xyb.y as f32;
        xyb_chunk[2] = xyb.b as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let test_colors = [
            (255, 0, 0),   // Red
            (0, 255, 0),   // Green
            (0, 0, 255),   // Blue
            (255, 255, 255), // White
            (0, 0, 0),     // Black
            (128, 128, 128), // Gray
        ];

        for (r, g, b) in test_colors {
            let xyb = srgb_to_xyb(r, g, b);
            let (r2, g2, b2) = xyb_to_srgb(&xyb);

            assert!(
                (r as i32 - r2 as i32).abs() <= 1,
                "R mismatch: {} -> {}",
                r,
                r2
            );
            assert!(
                (g as i32 - g2 as i32).abs() <= 1,
                "G mismatch: {} -> {}",
                g,
                g2
            );
            assert!(
                (b as i32 - b2 as i32).abs() <= 1,
                "B mismatch: {} -> {}",
                b,
                b2
            );
        }
    }

    #[test]
    fn test_neutral_colors() {
        // Neutral colors should have X ≈ 0 (no red-green opponent)
        let white = srgb_to_xyb(255, 255, 255);
        let black = srgb_to_xyb(0, 0, 0);
        let gray = srgb_to_xyb(128, 128, 128);

        assert!(white.x.abs() < 0.001, "White X should be ~0");
        assert!(black.x.abs() < 0.001, "Black X should be ~0");
        assert!(gray.x.abs() < 0.001, "Gray X should be ~0");
    }

    #[test]
    fn test_red_green_opponent() {
        let red = srgb_to_xyb(255, 0, 0);
        let green = srgb_to_xyb(0, 255, 0);

        // Red should have positive X (L > M)
        assert!(red.x > 0.0, "Red X should be positive");
        // Green should have negative X (M > L)
        assert!(green.x < 0.0, "Green X should be negative");
    }

    #[test]
    fn test_luminance_ordering() {
        let white = srgb_to_xyb(255, 255, 255);
        let gray = srgb_to_xyb(128, 128, 128);
        let black = srgb_to_xyb(0, 0, 0);

        // Y channel should increase with brightness
        assert!(white.y > gray.y, "White Y > Gray Y");
        assert!(gray.y > black.y, "Gray Y > Black Y");
    }

    #[test]
    fn test_buffer_transform() {
        let rgb_in = [0.5f32, 0.3, 0.7, 0.1, 0.9, 0.2];
        let mut xyb_out = [0.0f32; 6];
        let mut rgb_back = [0.0f32; 6];

        linear_rgb_to_xyb_buffer(&rgb_in, &mut xyb_out);
        xyb_to_linear_rgb_buffer(&xyb_out, &mut rgb_back);

        for i in 0..6 {
            assert!(
                (rgb_in[i] - rgb_back[i]).abs() < 0.001,
                "Buffer round-trip mismatch at {}: {} vs {}",
                i,
                rgb_in[i],
                rgb_back[i]
            );
        }
    }
}
