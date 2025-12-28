//! ICC Profile Parser
//!
//! This module provides the main IccProfile struct for parsing and using ICC profiles.
//!
//! # Structure
//!
//! An ICC profile consists of:
//! 1. A 128-byte header
//! 2. A tag table listing all tags
//! 3. Tag data (may overlap/share data)
//!
//! # Usage
//!
//! ```ignore
//! let profile = IccProfile::parse(&bytes)?;
//! println!("Profile: {:?}", profile.description());
//! ```

use std::collections::HashMap;

use super::error::IccError;
use super::header::{IccHeader, MIN_PROFILE_SIZE};
use super::tags::TagData;
use super::types::TagSignature;

/// An ICC profile parsed from bytes
#[derive(Debug, Clone)]
pub struct IccProfile {
    /// Profile header (128 bytes)
    pub header: IccHeader,
    /// Tag table: signature -> parsed data
    pub tags: HashMap<u32, TagData>,
    /// Raw profile data (for tags that need re-parsing)
    raw_data: Vec<u8>,
}

/// Tag table entry (as stored in profile)
#[derive(Debug, Clone, Copy)]
struct TagTableEntry {
    /// Tag signature
    signature: u32,
    /// Offset from start of profile
    offset: u32,
    /// Size of tag data
    size: u32,
}

impl IccProfile {
    /// Parse an ICC profile from bytes
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        // Parse header
        let header = IccHeader::parse(data)?;

        // Validate header
        header.validate(data.len())?;

        // Parse tag table
        let tag_count = Self::parse_tag_count(data)?;
        let tag_entries = Self::parse_tag_table(data, tag_count)?;

        // Parse each tag
        let mut tags = HashMap::with_capacity(tag_entries.len());

        for entry in &tag_entries {
            // Validate bounds
            let end = entry.offset as usize + entry.size as usize;
            if end > data.len() {
                return Err(IccError::TagOutOfBounds {
                    tag: entry.signature,
                    offset: entry.offset,
                    size: entry.size,
                    profile_size: data.len(),
                });
            }

            // Parse tag data
            let tag_data = &data[entry.offset as usize..end];
            match TagData::parse(tag_data, entry.signature) {
                Ok(parsed) => {
                    tags.insert(entry.signature, parsed);
                }
                Err(_) => {
                    // Store as unknown if parsing fails
                    tags.insert(
                        entry.signature,
                        TagData::Unknown {
                            type_sig: if tag_data.len() >= 4 {
                                u32::from_be_bytes([
                                    tag_data[0],
                                    tag_data[1],
                                    tag_data[2],
                                    tag_data[3],
                                ])
                            } else {
                                0
                            },
                            data: tag_data.to_vec(),
                        },
                    );
                }
            }
        }

        Ok(Self {
            header,
            tags,
            raw_data: data.to_vec(),
        })
    }

    /// Get the number of tags in the profile
    fn parse_tag_count(data: &[u8]) -> Result<usize, IccError> {
        if data.len() < MIN_PROFILE_SIZE + 4 {
            return Err(IccError::TooSmall {
                expected: MIN_PROFILE_SIZE + 4,
                actual: data.len(),
            });
        }

        let count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize;
        Ok(count)
    }

    /// Parse the tag table
    fn parse_tag_table(data: &[u8], count: usize) -> Result<Vec<TagTableEntry>, IccError> {
        let table_start = 132; // After header (128) + tag count (4)
        let entry_size = 12; // signature(4) + offset(4) + size(4)
        let required_size = table_start + count * entry_size;

        if data.len() < required_size {
            return Err(IccError::TooSmall {
                expected: required_size,
                actual: data.len(),
            });
        }

        let mut entries = Vec::with_capacity(count);

        for i in 0..count {
            let offset = table_start + i * entry_size;

            let signature =
                u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
            let tag_offset = u32::from_be_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            let size = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);

            entries.push(TagTableEntry {
                signature,
                offset: tag_offset,
                size,
            });
        }

        Ok(entries)
    }

    /// Get a tag by signature
    pub fn get_tag(&self, sig: TagSignature) -> Option<&TagData> {
        self.tags.get(&sig.0)
    }

    /// Get profile description
    pub fn description(&self) -> Option<String> {
        self.get_tag(TagSignature::DESC)
            .and_then(|t| t.as_text())
            .map(|t| t.text.clone())
    }

    /// Get copyright text
    pub fn copyright(&self) -> Option<String> {
        self.get_tag(TagSignature::COPYRIGHT)
            .and_then(|t| t.as_text())
            .map(|t| t.text.clone())
    }

    /// Get red colorant XYZ
    pub fn red_colorant(&self) -> Option<crate::color::Xyz> {
        self.get_tag(TagSignature::RED_COLORANT)
            .and_then(|t| t.as_xyz())
            .and_then(|xyz| xyz.to_xyz())
    }

    /// Get green colorant XYZ
    pub fn green_colorant(&self) -> Option<crate::color::Xyz> {
        self.get_tag(TagSignature::GREEN_COLORANT)
            .and_then(|t| t.as_xyz())
            .and_then(|xyz| xyz.to_xyz())
    }

    /// Get blue colorant XYZ
    pub fn blue_colorant(&self) -> Option<crate::color::Xyz> {
        self.get_tag(TagSignature::BLUE_COLORANT)
            .and_then(|t| t.as_xyz())
            .and_then(|xyz| xyz.to_xyz())
    }

    /// Get media white point
    pub fn media_white_point(&self) -> Option<crate::color::Xyz> {
        self.get_tag(TagSignature::MEDIA_WHITE)
            .and_then(|t| t.as_xyz())
            .and_then(|xyz| xyz.to_xyz())
    }

    /// Get media black point (bkpt tag)
    pub fn media_black_point(&self) -> Option<crate::color::Xyz> {
        self.get_tag(TagSignature::MEDIA_BLACK)
            .and_then(|t| t.as_xyz())
            .and_then(|xyz| xyz.to_xyz())
    }

    /// Get red TRC (tone reproduction curve)
    pub fn red_trc(&self) -> Option<&super::tags::CurveData> {
        self.get_tag(TagSignature::RED_TRC)
            .and_then(|t| t.as_curve())
    }

    /// Get green TRC
    pub fn green_trc(&self) -> Option<&super::tags::CurveData> {
        self.get_tag(TagSignature::GREEN_TRC)
            .and_then(|t| t.as_curve())
    }

    /// Get blue TRC
    pub fn blue_trc(&self) -> Option<&super::tags::CurveData> {
        self.get_tag(TagSignature::BLUE_TRC)
            .and_then(|t| t.as_curve())
    }

    /// Get gray TRC (for monochrome profiles)
    pub fn gray_trc(&self) -> Option<&super::tags::CurveData> {
        self.get_tag(TagSignature::GRAY_TRC)
            .and_then(|t| t.as_curve())
    }

    /// Check if this is a matrix-shaper profile (has colorants + TRCs)
    pub fn is_matrix_shaper(&self) -> bool {
        self.header.is_matrix_shaper()
            && self.get_tag(TagSignature::RED_COLORANT).is_some()
            && self.get_tag(TagSignature::GREEN_COLORANT).is_some()
            && self.get_tag(TagSignature::BLUE_COLORANT).is_some()
            && self.get_tag(TagSignature::RED_TRC).is_some()
            && self.get_tag(TagSignature::GREEN_TRC).is_some()
            && self.get_tag(TagSignature::BLUE_TRC).is_some()
    }

    /// Check if this is a LUT-based profile
    pub fn is_lut_based(&self) -> bool {
        self.get_tag(TagSignature::A2B0).is_some() || self.get_tag(TagSignature::B2A0).is_some()
    }

    /// Check if this is a CMYK profile
    pub fn is_cmyk(&self) -> bool {
        self.header.color_space == super::header::ColorSpace::Cmyk
    }

    /// Get the A2B0 tag (device to PCS, perceptual intent)
    pub fn a2b0(&self) -> Option<&TagData> {
        self.get_tag(TagSignature::A2B0)
    }

    /// Get the A2B1 tag (device to PCS, relative colorimetric intent)
    pub fn a2b1(&self) -> Option<&TagData> {
        self.get_tag(TagSignature::A2B1)
    }

    /// Get the A2B2 tag (device to PCS, saturation intent)
    pub fn a2b2(&self) -> Option<&TagData> {
        self.get_tag(TagSignature::A2B2)
    }

    /// Get the B2A0 tag (PCS to device, perceptual intent)
    pub fn b2a0(&self) -> Option<&TagData> {
        self.get_tag(TagSignature::B2A0)
    }

    /// Get the B2A1 tag (PCS to device, relative colorimetric intent)
    pub fn b2a1(&self) -> Option<&TagData> {
        self.get_tag(TagSignature::B2A1)
    }

    /// Get the B2A2 tag (PCS to device, saturation intent)
    pub fn b2a2(&self) -> Option<&TagData> {
        self.get_tag(TagSignature::B2A2)
    }

    /// Get the A2B tag for a specific rendering intent
    pub fn a2b_for_intent(&self, intent: super::header::RenderingIntent) -> Option<&TagData> {
        match intent {
            super::header::RenderingIntent::Perceptual => self.a2b0(),
            super::header::RenderingIntent::RelativeColorimetric
            | super::header::RenderingIntent::AbsoluteColorimetric => {
                self.a2b1().or_else(|| self.a2b0())
            }
            super::header::RenderingIntent::Saturation => self.a2b2().or_else(|| self.a2b0()),
        }
    }

    /// Get the B2A tag for a specific rendering intent
    pub fn b2a_for_intent(&self, intent: super::header::RenderingIntent) -> Option<&TagData> {
        match intent {
            super::header::RenderingIntent::Perceptual => self.b2a0(),
            super::header::RenderingIntent::RelativeColorimetric
            | super::header::RenderingIntent::AbsoluteColorimetric => {
                self.b2a1().or_else(|| self.b2a0())
            }
            super::header::RenderingIntent::Saturation => self.b2a2().or_else(|| self.b2a0()),
        }
    }

    /// Get the number of input channels for this profile
    pub fn input_channels(&self) -> usize {
        self.header.color_space.channels()
    }

    /// Get the number of output channels for PCS
    pub fn pcs_channels(&self) -> usize {
        self.header.pcs.channels()
    }

    /// Get the raw profile data
    pub fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    /// Get number of tags
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Iterate over tag signatures
    pub fn tag_signatures(&self) -> impl Iterator<Item = TagSignature> + '_ {
        self.tags.keys().map(|&sig| TagSignature(sig))
    }

    /// Get chromatic adaptation matrix (chad tag)
    pub fn chromatic_adaptation_matrix(&self) -> Option<[[f64; 3]; 3]> {
        match self.get_tag(TagSignature::CHAD)? {
            TagData::ChromaticAdaptation(matrix) => Some(*matrix),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icc::header::PROFILE_SIGNATURE;

    /// Create a minimal valid ICC profile for testing
    fn create_minimal_profile() -> Vec<u8> {
        let mut data = vec![0u8; 128 + 4]; // Header + tag count

        // Size
        let size = data.len() as u32;
        data[0..4].copy_from_slice(&size.to_be_bytes());

        // Version 4.3
        data[8] = 4;
        data[9] = 0x30;

        // Device class: Display
        data[12..16].copy_from_slice(b"mntr");

        // Color space: RGB
        data[16..20].copy_from_slice(b"RGB ");

        // PCS: XYZ
        data[20..24].copy_from_slice(b"XYZ ");

        // Signature: 'acsp'
        data[36..40].copy_from_slice(&PROFILE_SIGNATURE.to_be_bytes());

        // Tag count: 0
        data[128..132].copy_from_slice(&0u32.to_be_bytes());

        data
    }

    #[test]
    fn test_parse_minimal_profile() {
        let data = create_minimal_profile();
        let profile = IccProfile::parse(&data).unwrap();

        assert_eq!(profile.header.version.major, 4);
        assert_eq!(profile.tag_count(), 0);
    }

    #[test]
    fn test_profile_too_small() {
        let data = vec![0u8; 100];
        let result = IccProfile::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_with_one_tag() {
        let mut data = create_minimal_profile();

        // Update size
        let new_size = (data.len() + 12 + 20) as u32; // +12 tag entry +20 tag data
        data[0..4].copy_from_slice(&new_size.to_be_bytes());

        // Tag count: 1
        data[128..132].copy_from_slice(&1u32.to_be_bytes());

        // Tag entry: cprt (copyright), offset 144, size 20
        data.extend_from_slice(b"cprt"); // signature
        data.extend_from_slice(&144u32.to_be_bytes()); // offset
        data.extend_from_slice(&20u32.to_be_bytes()); // size

        // Tag data at offset 144
        data.extend_from_slice(b"text"); // type signature
        data.extend_from_slice(&[0u8; 4]); // reserved
        data.extend_from_slice(b"Test\0"); // text + null
        data.extend_from_slice(&[0u8; 7]); // padding to 20 bytes

        let profile = IccProfile::parse(&data).unwrap();
        assert_eq!(profile.tag_count(), 1);

        let copyright = profile.copyright();
        assert_eq!(copyright, Some("Test".to_string()));
    }
}
