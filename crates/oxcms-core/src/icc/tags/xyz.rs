//! XYZ Tag Type
//!
//! The XYZType contains an array of XYZ values. Used for colorant tags,
//! white point, black point, etc.
//!
//! See ICC.1:2022 Section 10.31

use crate::color::Xyz;
use crate::icc::error::IccError;
use crate::icc::types::XyzNumber;

/// XYZ tag data - contains one or more XYZ values
#[derive(Debug, Clone)]
pub struct XyzTagData {
    /// XYZ values stored in the tag
    pub values: Vec<XyzNumber>,
}

impl XyzTagData {
    /// Parse XYZ data from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        // Each XYZNumber is 12 bytes (3 × s15Fixed16)
        if data.len() < 12 {
            return Err(IccError::CorruptedData("XYZ tag too small".to_string()));
        }

        let count = data.len() / 12;
        let mut values = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 12;
            if let Some(xyz) = XyzNumber::from_bytes(&data[offset..offset + 12]) {
                values.push(xyz);
            }
        }

        if values.is_empty() {
            return Err(IccError::CorruptedData(
                "XYZ tag has no valid values".to_string(),
            ));
        }

        Ok(Self { values })
    }

    /// Get the first XYZ value (most common case)
    pub fn first(&self) -> Option<&XyzNumber> {
        self.values.first()
    }

    /// Get the first XYZ value as Xyz color type
    pub fn to_xyz(&self) -> Option<Xyz> {
        self.values.first().map(|v| v.to_xyz())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xyz_tag() {
        // D50 white point
        let data: [u8; 12] = [
            0x00, 0x00, 0xF6, 0xD6, // X = 0.9642 (approximately)
            0x00, 0x01, 0x00, 0x00, // Y = 1.0
            0x00, 0x00, 0xD3, 0x2D, // Z = 0.8249 (approximately)
        ];

        let tag = XyzTagData::parse(&data).unwrap();
        assert_eq!(tag.values.len(), 1);

        let xyz = tag.to_xyz().unwrap();
        assert!((xyz.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_multiple_xyz() {
        // Two XYZ values
        let data: [u8; 24] = [
            // First XYZ (1.0, 0.0, 0.0)
            0x00, 0x01, 0x00, 0x00, // X = 1.0
            0x00, 0x00, 0x00, 0x00, // Y = 0.0
            0x00, 0x00, 0x00, 0x00, // Z = 0.0
            // Second XYZ (0.0, 1.0, 0.0)
            0x00, 0x00, 0x00, 0x00, // X = 0.0
            0x00, 0x01, 0x00, 0x00, // Y = 1.0
            0x00, 0x00, 0x00, 0x00, // Z = 0.0
        ];

        let tag = XyzTagData::parse(&data).unwrap();
        assert_eq!(tag.values.len(), 2);
    }

    #[test]
    fn test_parse_xyz_too_small() {
        let data: [u8; 4] = [0, 0, 0, 0];
        assert!(XyzTagData::parse(&data).is_err());
    }
}
