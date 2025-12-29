//! ICC Profile Header
//!
//! The ICC profile header is exactly 128 bytes and contains basic profile information.
//! See ICC.1:2022 Section 7.2.

use super::error::IccError;
use super::types::{DateTimeNumber, XyzNumber};

/// Profile file signature - must be 'acsp' (0x61637370)
pub const PROFILE_SIGNATURE: u32 = 0x61637370;

/// Minimum valid profile size (header only)
pub const MIN_PROFILE_SIZE: usize = 128;

/// ICC Profile Header (128 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct IccHeader {
    /// Profile size in bytes
    pub size: u32,
    /// Preferred CMM type signature
    pub cmm_type: u32,
    /// Profile version (major.minor.0.0)
    pub version: ProfileVersion,
    /// Device class (display, input, output, etc.)
    pub device_class: ProfileClass,
    /// Color space of data (RGB, CMYK, etc.)
    pub color_space: ColorSpace,
    /// Profile connection space (XYZ or Lab)
    pub pcs: ColorSpace,
    /// Date and time profile was created
    pub creation_date: DateTimeNumber,
    /// Profile file signature (must be 'acsp')
    pub signature: u32,
    /// Primary platform signature
    pub platform: u32,
    /// Profile flags
    pub flags: u32,
    /// Device manufacturer signature
    pub manufacturer: u32,
    /// Device model signature
    pub model: u32,
    /// Device attributes
    pub attributes: u64,
    /// Rendering intent
    pub rendering_intent: RenderingIntent,
    /// PCS illuminant (should be D50)
    pub illuminant: XyzNumber,
    /// Profile creator signature
    pub creator: u32,
    /// Profile ID (MD5 hash, or zero)
    pub profile_id: [u8; 16],
}

impl IccHeader {
    /// Parse header from bytes
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < MIN_PROFILE_SIZE {
            return Err(IccError::TooSmall {
                expected: MIN_PROFILE_SIZE,
                actual: data.len(),
            });
        }

        let size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let cmm_type = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        let version = ProfileVersion {
            major: data[8],
            minor: data[9] >> 4,
            patch: data[9] & 0x0F,
        };

        let device_class =
            ProfileClass::from_u32(u32::from_be_bytes([data[12], data[13], data[14], data[15]]))?;

        let color_space =
            ColorSpace::from_u32(u32::from_be_bytes([data[16], data[17], data[18], data[19]]))?;

        let pcs =
            ColorSpace::from_u32(u32::from_be_bytes([data[20], data[21], data[22], data[23]]))?;

        let creation_date = DateTimeNumber::from_bytes(&data[24..36]).unwrap_or_default();

        let signature = u32::from_be_bytes([data[36], data[37], data[38], data[39]]);

        if signature != PROFILE_SIGNATURE {
            return Err(IccError::InvalidSignature(signature));
        }

        let platform = u32::from_be_bytes([data[40], data[41], data[42], data[43]]);
        let flags = u32::from_be_bytes([data[44], data[45], data[46], data[47]]);
        let manufacturer = u32::from_be_bytes([data[48], data[49], data[50], data[51]]);
        let model = u32::from_be_bytes([data[52], data[53], data[54], data[55]]);

        let attributes = u64::from_be_bytes([
            data[56], data[57], data[58], data[59], data[60], data[61], data[62], data[63],
        ]);

        let intent_value = u32::from_be_bytes([data[64], data[65], data[66], data[67]]);
        let rendering_intent = RenderingIntent::from_u32(intent_value)?;

        let illuminant = XyzNumber::from_bytes(&data[68..80]).unwrap_or_default();

        let creator = u32::from_be_bytes([data[80], data[81], data[82], data[83]]);

        let mut profile_id = [0u8; 16];
        profile_id.copy_from_slice(&data[84..100]);

        Ok(Self {
            size,
            cmm_type,
            version,
            device_class,
            color_space,
            pcs,
            creation_date,
            signature,
            platform,
            flags,
            manufacturer,
            model,
            attributes,
            rendering_intent,
            illuminant,
            creator,
            profile_id,
        })
    }

    /// Check if this is a valid header
    pub fn validate(&self, data_len: usize) -> Result<(), IccError> {
        if self.signature != PROFILE_SIGNATURE {
            return Err(IccError::InvalidSignature(self.signature));
        }

        if self.size as usize > data_len {
            return Err(IccError::SizeMismatch {
                header_size: self.size,
                actual_size: data_len,
            });
        }

        Ok(())
    }

    /// Check if this is a matrix/TRC profile
    pub fn is_matrix_shaper(&self) -> bool {
        matches!(
            self.device_class,
            ProfileClass::Display | ProfileClass::Input | ProfileClass::Output
        ) && matches!(self.color_space, ColorSpace::Rgb)
    }

    /// Get the version as a tuple (major, minor, patch)
    pub fn version_tuple(&self) -> (u8, u8, u8) {
        (self.version.major, self.version.minor, self.version.patch)
    }
}

/// ICC Profile Version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProfileVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl ProfileVersion {
    /// Check if version is at least the specified version
    pub fn at_least(&self, major: u8, minor: u8) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    /// Check if this is a v4 profile
    pub fn is_v4(&self) -> bool {
        self.major == 4
    }

    /// Check if this is a v2 profile
    pub fn is_v2(&self) -> bool {
        self.major == 2
    }
}

/// ICC Profile Class (Device Class)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileClass {
    /// Input device (scanner, camera)
    Input,
    /// Display device (monitor)
    Display,
    /// Output device (printer)
    Output,
    /// Device link
    DeviceLink,
    /// Color space conversion
    ColorSpace,
    /// Abstract profile
    Abstract,
    /// Named color profile
    NamedColor,
}

impl ProfileClass {
    pub fn from_u32(val: u32) -> Result<Self, IccError> {
        match &val.to_be_bytes() {
            b"scnr" => Ok(Self::Input),
            b"mntr" => Ok(Self::Display),
            b"prtr" => Ok(Self::Output),
            b"link" => Ok(Self::DeviceLink),
            b"spac" => Ok(Self::ColorSpace),
            b"abst" => Ok(Self::Abstract),
            b"nmcl" => Ok(Self::NamedColor),
            _ => Err(IccError::InvalidProfileClass(val)),
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            Self::Input => u32::from_be_bytes(*b"scnr"),
            Self::Display => u32::from_be_bytes(*b"mntr"),
            Self::Output => u32::from_be_bytes(*b"prtr"),
            Self::DeviceLink => u32::from_be_bytes(*b"link"),
            Self::ColorSpace => u32::from_be_bytes(*b"spac"),
            Self::Abstract => u32::from_be_bytes(*b"abst"),
            Self::NamedColor => u32::from_be_bytes(*b"nmcl"),
        }
    }
}

/// ICC Color Space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// XYZ
    Xyz,
    /// Lab
    Lab,
    /// Luv
    Luv,
    /// YCbCr
    YCbCr,
    /// Yxy
    Yxy,
    /// RGB
    Rgb,
    /// Grayscale
    Gray,
    /// HSV
    Hsv,
    /// HLS
    Hls,
    /// CMYK
    Cmyk,
    /// CMY
    Cmy,
    /// 2 color
    Color2,
    /// 3 color
    Color3,
    /// 4 color
    Color4,
    /// 5 color
    Color5,
    /// 6 color
    Color6,
    /// 7 color
    Color7,
    /// 8 color
    Color8,
    /// 9 color
    Color9,
    /// 10 color
    Color10,
    /// 11 color
    Color11,
    /// 12 color
    Color12,
    /// 13 color
    Color13,
    /// 14 color
    Color14,
    /// 15 color
    Color15,
}

impl ColorSpace {
    pub fn from_u32(val: u32) -> Result<Self, IccError> {
        match &val.to_be_bytes() {
            b"XYZ " => Ok(Self::Xyz),
            b"Lab " => Ok(Self::Lab),
            b"Luv " => Ok(Self::Luv),
            b"YCbr" => Ok(Self::YCbCr),
            b"Yxy " => Ok(Self::Yxy),
            b"RGB " => Ok(Self::Rgb),
            b"GRAY" => Ok(Self::Gray),
            b"HSV " => Ok(Self::Hsv),
            b"HLS " => Ok(Self::Hls),
            b"CMYK" => Ok(Self::Cmyk),
            b"CMY " => Ok(Self::Cmy),
            b"2CLR" => Ok(Self::Color2),
            b"3CLR" => Ok(Self::Color3),
            b"4CLR" => Ok(Self::Color4),
            b"5CLR" => Ok(Self::Color5),
            b"6CLR" => Ok(Self::Color6),
            b"7CLR" => Ok(Self::Color7),
            b"8CLR" => Ok(Self::Color8),
            b"9CLR" => Ok(Self::Color9),
            b"ACLR" => Ok(Self::Color10),
            b"BCLR" => Ok(Self::Color11),
            b"CCLR" => Ok(Self::Color12),
            b"DCLR" => Ok(Self::Color13),
            b"ECLR" => Ok(Self::Color14),
            b"FCLR" => Ok(Self::Color15),
            _ => Err(IccError::InvalidColorSpace(val)),
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            Self::Xyz => u32::from_be_bytes(*b"XYZ "),
            Self::Lab => u32::from_be_bytes(*b"Lab "),
            Self::Luv => u32::from_be_bytes(*b"Luv "),
            Self::YCbCr => u32::from_be_bytes(*b"YCbr"),
            Self::Yxy => u32::from_be_bytes(*b"Yxy "),
            Self::Rgb => u32::from_be_bytes(*b"RGB "),
            Self::Gray => u32::from_be_bytes(*b"GRAY"),
            Self::Hsv => u32::from_be_bytes(*b"HSV "),
            Self::Hls => u32::from_be_bytes(*b"HLS "),
            Self::Cmyk => u32::from_be_bytes(*b"CMYK"),
            Self::Cmy => u32::from_be_bytes(*b"CMY "),
            Self::Color2 => u32::from_be_bytes(*b"2CLR"),
            Self::Color3 => u32::from_be_bytes(*b"3CLR"),
            Self::Color4 => u32::from_be_bytes(*b"4CLR"),
            Self::Color5 => u32::from_be_bytes(*b"5CLR"),
            Self::Color6 => u32::from_be_bytes(*b"6CLR"),
            Self::Color7 => u32::from_be_bytes(*b"7CLR"),
            Self::Color8 => u32::from_be_bytes(*b"8CLR"),
            Self::Color9 => u32::from_be_bytes(*b"9CLR"),
            Self::Color10 => u32::from_be_bytes(*b"ACLR"),
            Self::Color11 => u32::from_be_bytes(*b"BCLR"),
            Self::Color12 => u32::from_be_bytes(*b"CCLR"),
            Self::Color13 => u32::from_be_bytes(*b"DCLR"),
            Self::Color14 => u32::from_be_bytes(*b"ECLR"),
            Self::Color15 => u32::from_be_bytes(*b"FCLR"),
        }
    }

    /// Get number of channels for this color space
    pub fn channels(&self) -> usize {
        match self {
            Self::Gray => 1,
            Self::Color2 => 2,
            Self::Xyz
            | Self::Lab
            | Self::Luv
            | Self::YCbCr
            | Self::Yxy
            | Self::Rgb
            | Self::Hsv
            | Self::Hls
            | Self::Cmy
            | Self::Color3 => 3,
            Self::Cmyk | Self::Color4 => 4,
            Self::Color5 => 5,
            Self::Color6 => 6,
            Self::Color7 => 7,
            Self::Color8 => 8,
            Self::Color9 => 9,
            Self::Color10 => 10,
            Self::Color11 => 11,
            Self::Color12 => 12,
            Self::Color13 => 13,
            Self::Color14 => 14,
            Self::Color15 => 15,
        }
    }
}

/// ICC Rendering Intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderingIntent {
    /// Perceptual - best for photographs
    #[default]
    Perceptual,
    /// Relative colorimetric - preserves in-gamut colors
    RelativeColorimetric,
    /// Saturation - maintains saturation
    Saturation,
    /// Absolute colorimetric - preserves white point
    AbsoluteColorimetric,
}

impl RenderingIntent {
    pub fn from_u32(val: u32) -> Result<Self, IccError> {
        match val {
            0 => Ok(Self::Perceptual),
            1 => Ok(Self::RelativeColorimetric),
            2 => Ok(Self::Saturation),
            3 => Ok(Self::AbsoluteColorimetric),
            _ => Err(IccError::InvalidRenderingIntent(val)),
        }
    }

    pub fn to_u32(&self) -> u32 {
        match self {
            Self::Perceptual => 0,
            Self::RelativeColorimetric => 1,
            Self::Saturation => 2,
            Self::AbsoluteColorimetric => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_space_channels() {
        assert_eq!(ColorSpace::Gray.channels(), 1);
        assert_eq!(ColorSpace::Rgb.channels(), 3);
        assert_eq!(ColorSpace::Cmyk.channels(), 4);
    }

    #[test]
    fn test_profile_class_roundtrip() {
        for class in [
            ProfileClass::Input,
            ProfileClass::Display,
            ProfileClass::Output,
            ProfileClass::DeviceLink,
        ] {
            let val = class.to_u32();
            let back = ProfileClass::from_u32(val).unwrap();
            assert_eq!(class, back);
        }
    }

    #[test]
    fn test_rendering_intent() {
        for i in 0..4 {
            let intent = RenderingIntent::from_u32(i).unwrap();
            assert_eq!(intent.to_u32(), i);
        }
        assert!(RenderingIntent::from_u32(4).is_err());
    }

    #[test]
    fn test_profile_version() {
        let v2 = ProfileVersion {
            major: 2,
            minor: 4,
            patch: 0,
        };
        assert!(v2.is_v2());
        assert!(!v2.is_v4());
        assert!(v2.at_least(2, 0));
        assert!(v2.at_least(2, 4));
        assert!(!v2.at_least(2, 5));

        let v4 = ProfileVersion {
            major: 4,
            minor: 3,
            patch: 0,
        };
        assert!(v4.is_v4());
        assert!(v4.at_least(2, 0));
        assert!(v4.at_least(4, 3));
    }
}
