//! ICC Profile Tag Parsing
//!
//! Tags contain the actual profile data. Each tag has:
//! - A 4-byte signature identifying the tag
//! - A 4-byte type signature identifying the data format
//! - Reserved bytes
//! - Type-specific data
//!
//! See ICC.1:2022 Section 9.

mod curves;
mod lut;
mod text;
mod xyz;

pub use curves::{CurveData, ParametricCurveData};
pub use lut::{CurveSegment, Lut8Data, Lut16Data, LutAToBData, LutBToAData, LutClut};
pub use text::TextData;
pub use xyz::XyzTagData;

use super::error::IccError;
use super::types::TypeSignature;

/// Parsed tag data
#[derive(Debug, Clone)]
pub enum TagData {
    /// XYZ type data (colorants, white point)
    Xyz(XyzTagData),
    /// Curve type (TRC)
    Curve(CurveData),
    /// Parametric curve type
    ParametricCurve(ParametricCurveData),
    /// Text description
    Text(TextData),
    /// Multi-localized Unicode text
    MultiLocalizedUnicode(TextData),
    /// 8-bit LUT
    Lut8(Lut8Data),
    /// 16-bit LUT
    Lut16(Lut16Data),
    /// LUT A to B
    LutAToB(LutAToBData),
    /// LUT B to A
    LutBToA(LutBToAData),
    /// Chromatic adaptation matrix (sf32)
    ChromaticAdaptation([[f64; 3]; 3]),
    /// Unknown/unsupported tag type
    Unknown { type_sig: u32, data: Vec<u8> },
}

impl TagData {
    /// Parse tag data from bytes
    ///
    /// # Arguments
    /// * `data` - The tag data bytes (starting at offset in profile)
    /// * `tag_sig` - The tag signature (for context-specific parsing)
    pub fn parse(data: &[u8], _tag_sig: u32) -> Result<Self, IccError> {
        if data.len() < 8 {
            return Err(IccError::CorruptedData(
                "Tag data too small for header".to_string(),
            ));
        }

        let type_sig = TypeSignature(u32::from_be_bytes([data[0], data[1], data[2], data[3]]));
        // Bytes 4-7 are reserved (should be 0)

        let type_data = &data[8..];

        match type_sig {
            TypeSignature::XYZ => {
                let xyz = XyzTagData::parse(type_data)?;
                Ok(TagData::Xyz(xyz))
            }
            TypeSignature::CURVE => {
                let curve = CurveData::parse(type_data)?;
                Ok(TagData::Curve(curve))
            }
            TypeSignature::PARA => {
                let curve = ParametricCurveData::parse(type_data)?;
                Ok(TagData::ParametricCurve(curve))
            }
            TypeSignature::TEXT => {
                let text = TextData::parse_text(type_data)?;
                Ok(TagData::Text(text))
            }
            TypeSignature::DESC => {
                let text = TextData::parse_desc(type_data)?;
                Ok(TagData::Text(text))
            }
            TypeSignature::MLUC => {
                let text = TextData::parse_mluc(type_data)?;
                Ok(TagData::MultiLocalizedUnicode(text))
            }
            TypeSignature::LUT8 => {
                let lut = Lut8Data::parse(type_data)?;
                Ok(TagData::Lut8(lut))
            }
            TypeSignature::LUT16 => {
                let lut = Lut16Data::parse(type_data)?;
                Ok(TagData::Lut16(lut))
            }
            TypeSignature::LUTA2B => {
                let lut = LutAToBData::parse(type_data)?;
                Ok(TagData::LutAToB(lut))
            }
            TypeSignature::LUTB2A => {
                let lut = LutBToAData::parse(type_data)?;
                Ok(TagData::LutBToA(lut))
            }
            TypeSignature::SF32 => {
                // sf32 is used for chromatic adaptation matrix
                let matrix = parse_sf32_matrix(type_data)?;
                Ok(TagData::ChromaticAdaptation(matrix))
            }
            _ => {
                // Unknown type - store raw data
                Ok(TagData::Unknown {
                    type_sig: type_sig.0,
                    data: data.to_vec(),
                })
            }
        }
    }

    /// Check if this is an XYZ tag
    pub fn as_xyz(&self) -> Option<&XyzTagData> {
        match self {
            TagData::Xyz(xyz) => Some(xyz),
            _ => None,
        }
    }

    /// Check if this is a curve tag
    pub fn as_curve(&self) -> Option<&CurveData> {
        match self {
            TagData::Curve(curve) => Some(curve),
            _ => None,
        }
    }

    /// Check if this is a parametric curve tag
    pub fn as_parametric_curve(&self) -> Option<&ParametricCurveData> {
        match self {
            TagData::ParametricCurve(curve) => Some(curve),
            _ => None,
        }
    }

    /// Check if this is a text tag
    pub fn as_text(&self) -> Option<&TextData> {
        match self {
            TagData::Text(text) | TagData::MultiLocalizedUnicode(text) => Some(text),
            _ => None,
        }
    }

    /// Get as Lut8 data
    pub fn as_lut8(&self) -> Option<&Lut8Data> {
        match self {
            TagData::Lut8(lut) => Some(lut),
            _ => None,
        }
    }

    /// Get as Lut16 data
    pub fn as_lut16(&self) -> Option<&Lut16Data> {
        match self {
            TagData::Lut16(lut) => Some(lut),
            _ => None,
        }
    }

    /// Get as LutAToB data
    pub fn as_lut_a2b(&self) -> Option<&LutAToBData> {
        match self {
            TagData::LutAToB(lut) => Some(lut),
            _ => None,
        }
    }

    /// Get as LutBToA data
    pub fn as_lut_b2a(&self) -> Option<&LutBToAData> {
        match self {
            TagData::LutBToA(lut) => Some(lut),
            _ => None,
        }
    }

    /// Check if this is any kind of LUT tag
    pub fn is_lut(&self) -> bool {
        matches!(
            self,
            TagData::Lut8(_) | TagData::Lut16(_) | TagData::LutAToB(_) | TagData::LutBToA(_)
        )
    }
}

/// Parse sf32 type as 3x3 matrix
fn parse_sf32_matrix(data: &[u8]) -> Result<[[f64; 3]; 3], IccError> {
    if data.len() < 36 {
        return Err(IccError::CorruptedData("sf32 matrix too small".to_string()));
    }

    let mut matrix = [[0.0f64; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            let offset = (row * 3 + col) * 4;
            let raw = i32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            matrix[row][col] = raw as f64 / 65536.0;
        }
    }

    Ok(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unknown_type() {
        // Unknown type signature
        let data = [
            b'u', b'n', b'k', b'n', // type sig "unkn"
            0, 0, 0, 0, // reserved
            1, 2, 3, 4, // payload
        ];

        let tag = TagData::parse(&data, 0).unwrap();
        match tag {
            TagData::Unknown { type_sig, .. } => {
                assert_eq!(type_sig, u32::from_be_bytes(*b"unkn"));
            }
            _ => panic!("Expected Unknown tag type"),
        }
    }
}
