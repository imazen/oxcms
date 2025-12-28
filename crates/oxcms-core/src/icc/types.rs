//! ICC Profile Basic Types
//!
//! These types match the ICC.1:2022 specification exactly.

use crate::color::Xyz;

/// ICC Tag Signature (4-byte ASCII code)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TagSignature(pub u32);

impl TagSignature {
    /// Create from 4 ASCII characters
    pub const fn from_bytes(b: [u8; 4]) -> Self {
        Self(u32::from_be_bytes(b))
    }

    /// Convert to ASCII string (if valid)
    pub fn to_string(&self) -> String {
        let bytes = self.0.to_be_bytes();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // Common tag signatures
    pub const A2B0: Self = Self::from_bytes(*b"A2B0");
    pub const A2B1: Self = Self::from_bytes(*b"A2B1");
    pub const A2B2: Self = Self::from_bytes(*b"A2B2");
    pub const B2A0: Self = Self::from_bytes(*b"B2A0");
    pub const B2A1: Self = Self::from_bytes(*b"B2A1");
    pub const B2A2: Self = Self::from_bytes(*b"B2A2");
    pub const BLUE_COLORANT: Self = Self::from_bytes(*b"bXYZ");
    pub const BLUE_TRC: Self = Self::from_bytes(*b"bTRC");
    pub const CHAD: Self = Self::from_bytes(*b"chad");
    pub const COPYRIGHT: Self = Self::from_bytes(*b"cprt");
    pub const DESC: Self = Self::from_bytes(*b"desc");
    pub const DMDD: Self = Self::from_bytes(*b"dmdd");
    pub const DMND: Self = Self::from_bytes(*b"dmnd");
    pub const GAMUT: Self = Self::from_bytes(*b"gamt");
    pub const GRAY_TRC: Self = Self::from_bytes(*b"kTRC");
    pub const GREEN_COLORANT: Self = Self::from_bytes(*b"gXYZ");
    pub const GREEN_TRC: Self = Self::from_bytes(*b"gTRC");
    pub const LUMINANCE: Self = Self::from_bytes(*b"lumi");
    pub const MEDIA_WHITE: Self = Self::from_bytes(*b"wtpt");
    pub const MEDIA_BLACK: Self = Self::from_bytes(*b"bkpt");
    pub const PREVIEW0: Self = Self::from_bytes(*b"pre0");
    pub const PREVIEW1: Self = Self::from_bytes(*b"pre1");
    pub const PREVIEW2: Self = Self::from_bytes(*b"pre2");
    pub const PROFILE_DESC: Self = Self::from_bytes(*b"desc");
    pub const RED_COLORANT: Self = Self::from_bytes(*b"rXYZ");
    pub const RED_TRC: Self = Self::from_bytes(*b"rTRC");
    pub const TECH: Self = Self::from_bytes(*b"tech");
    pub const VIEW_COND_DESC: Self = Self::from_bytes(*b"vued");
    pub const VIEW_COND: Self = Self::from_bytes(*b"view");
}

/// Type signatures for ICC tag data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeSignature(pub u32);

impl TypeSignature {
    pub const fn from_bytes(b: [u8; 4]) -> Self {
        Self(u32::from_be_bytes(b))
    }

    // Common type signatures
    pub const XYZ: Self = Self::from_bytes(*b"XYZ ");
    pub const CURVE: Self = Self::from_bytes(*b"curv");
    pub const PARA: Self = Self::from_bytes(*b"para");
    pub const TEXT: Self = Self::from_bytes(*b"text");
    pub const DESC: Self = Self::from_bytes(*b"desc");
    pub const MLUC: Self = Self::from_bytes(*b"mluc");
    pub const LUT8: Self = Self::from_bytes(*b"mft1");
    pub const LUT16: Self = Self::from_bytes(*b"mft2");
    pub const LUTA2B: Self = Self::from_bytes(*b"mAB ");
    pub const LUTB2A: Self = Self::from_bytes(*b"mBA ");
    pub const SF32: Self = Self::from_bytes(*b"sf32");
    pub const CHAD: Self = Self::from_bytes(*b"sf32"); // chromatic adaptation uses sf32
    pub const SIG: Self = Self::from_bytes(*b"sig ");
}

/// s15Fixed16Number - 16.16 fixed point
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct S15Fixed16(pub i32);

impl S15Fixed16 {
    /// Create from raw i32 value
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Create from f64 value
    pub fn from_f64(val: f64) -> Self {
        Self((val * 65536.0) as i32)
    }

    /// Convert to f64
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / 65536.0
    }

    /// Parse from big-endian bytes
    pub fn from_be_bytes(bytes: [u8; 4]) -> Self {
        Self(i32::from_be_bytes(bytes))
    }
}

/// u16Fixed16Number - unsigned 16.16 fixed point
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U16Fixed16(pub u32);

impl U16Fixed16 {
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn from_f64(val: f64) -> Self {
        Self((val * 65536.0) as u32)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / 65536.0
    }

    pub fn from_be_bytes(bytes: [u8; 4]) -> Self {
        Self(u32::from_be_bytes(bytes))
    }
}

/// u8Fixed8Number - unsigned 8.8 fixed point
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U8Fixed8(pub u16);

impl U8Fixed8 {
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / 256.0
    }

    pub fn from_be_bytes(bytes: [u8; 2]) -> Self {
        Self(u16::from_be_bytes(bytes))
    }
}

/// XYZNumber - ICC XYZ value (3 × s15Fixed16)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct XyzNumber {
    pub x: S15Fixed16,
    pub y: S15Fixed16,
    pub z: S15Fixed16,
}

impl XyzNumber {
    /// Parse from 12 bytes (big-endian)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }
        Some(Self {
            x: S15Fixed16::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            y: S15Fixed16::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            z: S15Fixed16::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        })
    }

    /// Convert to Xyz color type
    pub fn to_xyz(&self) -> Xyz {
        Xyz::new(self.x.to_f64(), self.y.to_f64(), self.z.to_f64())
    }
}

/// dateTimeNumber - ICC date/time
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DateTimeNumber {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
}

impl DateTimeNumber {
    /// Parse from 12 bytes (big-endian)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 {
            return None;
        }
        Some(Self {
            year: u16::from_be_bytes([bytes[0], bytes[1]]),
            month: u16::from_be_bytes([bytes[2], bytes[3]]),
            day: u16::from_be_bytes([bytes[4], bytes[5]]),
            hour: u16::from_be_bytes([bytes[6], bytes[7]]),
            minute: u16::from_be_bytes([bytes[8], bytes[9]]),
            second: u16::from_be_bytes([bytes[10], bytes[11]]),
        })
    }
}

/// Response16Number for device calibration
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Response16 {
    pub device: u16,
    pub measurement: S15Fixed16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s15fixed16() {
        let one = S15Fixed16::from_f64(1.0);
        assert!((one.to_f64() - 1.0).abs() < 1e-6);

        let half = S15Fixed16::from_f64(0.5);
        assert!((half.to_f64() - 0.5).abs() < 1e-6);

        let neg = S15Fixed16::from_f64(-1.5);
        assert!((neg.to_f64() - (-1.5)).abs() < 1e-6);
    }

    #[test]
    fn test_xyz_number() {
        // D50 white point in ICC encoding
        let bytes: [u8; 12] = [
            0x00, 0x00, 0xF6, 0xD6, // X = 0.9642
            0x00, 0x01, 0x00, 0x00, // Y = 1.0
            0x00, 0x00, 0xD3, 0x2D, // Z = 0.8249
        ];
        let xyz = XyzNumber::from_bytes(&bytes).unwrap();
        let color = xyz.to_xyz();

        assert!((color.x - 0.9642).abs() < 0.001);
        assert!((color.y - 1.0).abs() < 0.001);
        assert!((color.z - 0.8249).abs() < 0.001);
    }

    #[test]
    fn test_tag_signature() {
        let desc = TagSignature::DESC;
        assert_eq!(desc.to_string(), "desc");

        let r_xyz = TagSignature::RED_COLORANT;
        assert_eq!(r_xyz.to_string(), "rXYZ");
    }
}
