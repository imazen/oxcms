//! Text Tag Types
//!
//! ICC profiles can contain text in several formats:
//! - text: Simple ASCII text
//! - desc: Profile description (v2 format)
//! - mluc: Multi-localized Unicode (v4 format)
//!
//! See ICC.1:2022 Sections 10.24 (text), 10.14 (desc), 10.15 (mluc)

use crate::icc::error::IccError;

/// Text tag data
#[derive(Debug, Clone)]
pub struct TextData {
    /// Primary text content (English or default)
    pub text: String,
    /// Localized versions (language code -> text)
    pub localized: Vec<(String, String)>,
}

impl TextData {
    /// Create from a single string
    pub fn new(text: String) -> Self {
        Self {
            text,
            localized: Vec::new(),
        }
    }

    /// Parse 'text' type (simple ASCII)
    pub fn parse_text(data: &[u8]) -> Result<Self, IccError> {
        // Text is null-terminated ASCII
        let text = data
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect::<String>();

        Ok(Self::new(text))
    }

    /// Parse 'desc' type (v2 profile description)
    pub fn parse_desc(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 4 {
            return Err(IccError::CorruptedData(
                "Description tag too small".to_string(),
            ));
        }

        // ASCII description count (includes null)
        let ascii_count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < 4 + ascii_count {
            return Err(IccError::CorruptedData(
                "Description ASCII data truncated".to_string(),
            ));
        }

        // Read ASCII string
        let ascii_data = &data[4..4 + ascii_count];
        let text = ascii_data
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as char)
            .collect::<String>();

        // Note: desc also contains Unicode and ScriptCode versions,
        // but they're rarely used and often broken in real profiles

        Ok(Self::new(text))
    }

    /// Parse 'mluc' type (multi-localized Unicode)
    pub fn parse_mluc(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 8 {
            return Err(IccError::CorruptedData(
                "mluc tag too small".to_string(),
            ));
        }

        let record_count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let record_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;

        if record_size < 12 {
            return Err(IccError::CorruptedData(
                "mluc record size too small".to_string(),
            ));
        }

        let mut localized = Vec::with_capacity(record_count);
        let mut primary_text = String::new();

        for i in 0..record_count {
            let record_offset = 8 + i * record_size;
            if data.len() < record_offset + 12 {
                break;
            }

            let record = &data[record_offset..];

            // Language code (2 bytes)
            let lang = format!(
                "{}{}",
                record[0] as char,
                record[1] as char
            );

            // Country code (2 bytes)
            let country = format!(
                "{}{}",
                record[2] as char,
                record[3] as char
            );

            let locale = format!("{}-{}", lang, country);

            // String length and offset
            let str_len = u32::from_be_bytes([record[4], record[5], record[6], record[7]]) as usize;
            let str_offset =
                u32::from_be_bytes([record[8], record[9], record[10], record[11]]) as usize;

            // The offset is relative to the start of the tag type data (not the record)
            if str_offset + str_len <= data.len() && str_len >= 2 {
                // UTF-16BE string
                let utf16_data = &data[str_offset..str_offset + str_len];
                if let Some(text) = decode_utf16be(utf16_data) {
                    // Use first record as primary (usually en-US)
                    if primary_text.is_empty() {
                        primary_text = text.clone();
                    }
                    localized.push((locale, text));
                }
            }
        }

        if primary_text.is_empty() && !localized.is_empty() {
            primary_text = localized[0].1.clone();
        }

        Ok(Self {
            text: primary_text,
            localized,
        })
    }

    /// Get text for a specific locale
    pub fn get_locale(&self, lang: &str) -> Option<&str> {
        for (locale, text) in &self.localized {
            if locale.starts_with(lang) {
                return Some(text);
            }
        }
        None
    }
}

/// Decode UTF-16BE bytes to String
fn decode_utf16be(data: &[u8]) -> Option<String> {
    if data.len() % 2 != 0 {
        return None;
    }

    let utf16: Vec<u16> = data
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();

    // Remove null terminator if present
    let utf16: Vec<u16> = utf16.iter().take_while(|&&c| c != 0).copied().collect();

    String::from_utf16(&utf16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text() {
        let data = b"Hello, World!\0";
        let text = TextData::parse_text(data).unwrap();
        assert_eq!(text.text, "Hello, World!");
    }

    #[test]
    fn test_parse_desc() {
        // Count = 6 (including null)
        let mut data = vec![0, 0, 0, 6];
        data.extend_from_slice(b"sRGB\0\0"); // ASCII + null + padding

        let text = TextData::parse_desc(&data).unwrap();
        assert_eq!(text.text, "sRGB");
    }

    #[test]
    fn test_parse_mluc() {
        // 1 record, 12 bytes each
        let mut data = vec![
            0, 0, 0, 1, // record count = 1
            0, 0, 0, 12, // record size = 12
        ];

        // Record: en-US
        data.extend_from_slice(&[
            b'e', b'n', // language
            b'U', b'S', // country
            0, 0, 0, 10, // string length = 10 bytes (5 UTF-16 chars)
            0, 0, 0, 20, // string offset = 20 (after this record)
        ]);

        // UTF-16BE string "Test" (with null)
        data.extend_from_slice(&[0x00, b'T', 0x00, b'e', 0x00, b's', 0x00, b't', 0x00, 0x00]);

        let text = TextData::parse_mluc(&data).unwrap();
        assert_eq!(text.text, "Test");
        assert!(!text.localized.is_empty());
    }

    #[test]
    fn test_decode_utf16be() {
        // "Hello" in UTF-16BE
        let data = [0x00, 0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F];
        let result = decode_utf16be(&data).unwrap();
        assert_eq!(result, "Hello");
    }
}
