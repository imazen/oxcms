//! Public API types for oxcms
//!
//! These types provide a stable public API that doesn't expose moxcms internals.

/// Color space of profile data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorSpace {
    /// RGB color space
    Rgb,
    /// CMYK color space
    Cmyk,
    /// Grayscale
    Gray,
    /// CIELAB
    Lab,
    /// CIEXYZ
    Xyz,
    /// YCbCr (video)
    YCbCr,
    /// Luminance + Chroma (video)
    Luv,
    /// HSV (hue, saturation, value)
    Hsv,
    /// HLS (hue, lightness, saturation)
    Hls,
    /// CMY (without K)
    Cmy,
    /// Multi-channel (5+ channels)
    MultiChannel,
    /// Unknown or unsupported
    Unknown,
}

impl ColorSpace {
    /// Number of channels for this color space
    pub fn channels(&self) -> usize {
        match self {
            Self::Gray => 1,
            Self::Rgb | Self::Lab | Self::Xyz | Self::YCbCr | Self::Luv | Self::Hsv | Self::Hls => 3,
            Self::Cmyk | Self::Cmy => 4,
            Self::MultiChannel | Self::Unknown => 0, // Variable
        }
    }

    /// Check if this is an RGB-like color space
    pub fn is_rgb(&self) -> bool {
        matches!(self, Self::Rgb)
    }

    /// Check if this is a CMYK color space
    pub fn is_cmyk(&self) -> bool {
        matches!(self, Self::Cmyk)
    }

    /// Check if this is a grayscale color space
    pub fn is_gray(&self) -> bool {
        matches!(self, Self::Gray)
    }
}

impl From<moxcms::DataColorSpace> for ColorSpace {
    fn from(cs: moxcms::DataColorSpace) -> Self {
        match cs {
            moxcms::DataColorSpace::Rgb => Self::Rgb,
            moxcms::DataColorSpace::Cmyk => Self::Cmyk,
            moxcms::DataColorSpace::Gray => Self::Gray,
            moxcms::DataColorSpace::Lab => Self::Lab,
            moxcms::DataColorSpace::Xyz => Self::Xyz,
            moxcms::DataColorSpace::YCbr => Self::YCbCr,
            moxcms::DataColorSpace::Luv => Self::Luv,
            moxcms::DataColorSpace::Hsv => Self::Hsv,
            moxcms::DataColorSpace::Hls => Self::Hls,
            moxcms::DataColorSpace::Cmy => Self::Cmy,
            _ => Self::Unknown,
        }
    }
}

/// ICC profile class (device type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProfileClass {
    /// Input device (scanner, camera)
    Input,
    /// Display device (monitor)
    Display,
    /// Output device (printer)
    Output,
    /// Device link (direct device-to-device)
    DeviceLink,
    /// Color space conversion
    ColorSpace,
    /// Abstract profile
    Abstract,
    /// Named color profile
    NamedColor,
    /// Unknown class
    Unknown,
}

impl From<moxcms::ProfileClass> for ProfileClass {
    fn from(pc: moxcms::ProfileClass) -> Self {
        match pc {
            moxcms::ProfileClass::InputDevice => Self::Input,
            moxcms::ProfileClass::DisplayDevice => Self::Display,
            moxcms::ProfileClass::OutputDevice => Self::Output,
            moxcms::ProfileClass::DeviceLink => Self::DeviceLink,
            moxcms::ProfileClass::ColorSpace => Self::ColorSpace,
            moxcms::ProfileClass::Abstract => Self::Abstract,
            moxcms::ProfileClass::Named => Self::NamedColor,
        }
    }
}

/// ICC profile version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfileVersion {
    /// Major version (2 or 4)
    pub major: u8,
    /// Minor version
    pub minor: u8,
    /// Patch version
    pub patch: u8,
}

impl ProfileVersion {
    /// Create a new profile version
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self { major, minor, patch }
    }

    /// ICC v2.4
    pub const V2_4: Self = Self::new(2, 4, 0);
    /// ICC v4.3
    pub const V4_3: Self = Self::new(4, 3, 0);
    /// ICC v4.4
    pub const V4_4: Self = Self::new(4, 4, 0);

    /// Check if this is a v2 profile
    pub fn is_v2(&self) -> bool {
        self.major == 2
    }

    /// Check if this is a v4 profile
    pub fn is_v4(&self) -> bool {
        self.major == 4
    }
}

impl From<moxcms::ProfileVersion> for ProfileVersion {
    fn from(pv: moxcms::ProfileVersion) -> Self {
        match pv {
            moxcms::ProfileVersion::V2_0 => Self::new(2, 0, 0),
            moxcms::ProfileVersion::V2_1 => Self::new(2, 1, 0),
            moxcms::ProfileVersion::V2_2 => Self::new(2, 2, 0),
            moxcms::ProfileVersion::V2_3 => Self::new(2, 3, 0),
            moxcms::ProfileVersion::V2_4 => Self::new(2, 4, 0),
            moxcms::ProfileVersion::V4_0 => Self::new(4, 0, 0),
            moxcms::ProfileVersion::V4_1 => Self::new(4, 1, 0),
            moxcms::ProfileVersion::V4_2 => Self::new(4, 2, 0),
            moxcms::ProfileVersion::V4_3 => Self::new(4, 3, 0),
            moxcms::ProfileVersion::V4_4 => Self::new(4, 4, 0),
            moxcms::ProfileVersion::Unknown => Self::new(0, 0, 0),
        }
    }
}

impl std::fmt::Display for ProfileVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Rendering intent for color transformations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum RenderingIntent {
    /// Perceptual - compress gamut to fit, preserve relationships
    #[default]
    Perceptual,
    /// Relative colorimetric - map white point, clip out-of-gamut
    RelativeColorimetric,
    /// Saturation - preserve saturation over accuracy
    Saturation,
    /// Absolute colorimetric - no white point mapping
    AbsoluteColorimetric,
}

impl From<moxcms::RenderingIntent> for RenderingIntent {
    fn from(ri: moxcms::RenderingIntent) -> Self {
        match ri {
            moxcms::RenderingIntent::Perceptual => Self::Perceptual,
            moxcms::RenderingIntent::RelativeColorimetric => Self::RelativeColorimetric,
            moxcms::RenderingIntent::Saturation => Self::Saturation,
            moxcms::RenderingIntent::AbsoluteColorimetric => Self::AbsoluteColorimetric,
        }
    }
}

impl From<RenderingIntent> for moxcms::RenderingIntent {
    fn from(ri: RenderingIntent) -> Self {
        match ri {
            RenderingIntent::Perceptual => Self::Perceptual,
            RenderingIntent::RelativeColorimetric => Self::RelativeColorimetric,
            RenderingIntent::Saturation => Self::Saturation,
            RenderingIntent::AbsoluteColorimetric => Self::AbsoluteColorimetric,
        }
    }
}

/// CIE XYZ color value
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XyzColor {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl XyzColor {
    /// Create a new XYZ color
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// D50 white point (ICC PCS illuminant)
    pub const D50: Self = Self::new(0.9642, 1.0, 0.8249);

    /// D65 white point (sRGB, Display P3)
    pub const D65: Self = Self::new(0.95047, 1.0, 1.08883);
}

impl From<moxcms::Xyzd> for XyzColor {
    fn from(xyz: moxcms::Xyzd) -> Self {
        Self {
            x: xyz.x,
            y: xyz.y,
            z: xyz.z,
        }
    }
}

/// 3x3 matrix for color transformations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3x3 {
    /// Row-major matrix values
    pub m: [[f64; 3]; 3],
}

impl Matrix3x3 {
    /// Create a new matrix from row-major values
    pub const fn new(m: [[f64; 3]; 3]) -> Self {
        Self { m }
    }

    /// Identity matrix
    pub const IDENTITY: Self = Self::new([
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ]);

    /// Multiply matrix by a 3-element vector
    pub fn multiply_vec(&self, v: [f64; 3]) -> [f64; 3] {
        [
            self.m[0][0] * v[0] + self.m[0][1] * v[1] + self.m[0][2] * v[2],
            self.m[1][0] * v[0] + self.m[1][1] * v[1] + self.m[1][2] * v[2],
            self.m[2][0] * v[0] + self.m[2][1] * v[1] + self.m[2][2] * v[2],
        ]
    }
}

impl From<moxcms::Matrix3d> for Matrix3x3 {
    fn from(m: moxcms::Matrix3d) -> Self {
        Self {
            m: [
                [m.v[0][0], m.v[0][1], m.v[0][2]],
                [m.v[1][0], m.v[1][1], m.v[1][2]],
                [m.v[2][0], m.v[2][1], m.v[2][2]],
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_space_channels() {
        assert_eq!(ColorSpace::Rgb.channels(), 3);
        assert_eq!(ColorSpace::Cmyk.channels(), 4);
        assert_eq!(ColorSpace::Gray.channels(), 1);
        assert_eq!(ColorSpace::Lab.channels(), 3);
    }

    #[test]
    fn test_profile_version_display() {
        let v = ProfileVersion::new(4, 3, 0);
        assert_eq!(v.to_string(), "4.3.0");
    }

    #[test]
    fn test_matrix_multiply() {
        let m = Matrix3x3::IDENTITY;
        let v = [1.0, 2.0, 3.0];
        let result = m.multiply_vec(v);
        assert_eq!(result, v);
    }
}
