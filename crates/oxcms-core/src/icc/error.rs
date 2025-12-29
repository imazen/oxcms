//! ICC Profile Error Types

use std::fmt;

/// Errors that can occur when parsing ICC profiles
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum IccError {
    /// Profile data is too small
    TooSmall { expected: usize, actual: usize },
    /// Invalid profile signature (should be 'acsp')
    InvalidSignature(u32),
    /// Profile size in header doesn't match data
    SizeMismatch {
        header_size: u32,
        actual_size: usize,
    },
    /// Tag offset is out of bounds
    TagOutOfBounds {
        tag: u32,
        offset: u32,
        size: u32,
        profile_size: usize,
    },
    /// Invalid tag type signature
    InvalidTagType { tag: u32, type_sig: u32 },
    /// Required tag is missing
    MissingTag(u32),
    /// Invalid color space
    InvalidColorSpace(u32),
    /// Invalid profile class
    InvalidProfileClass(u32),
    /// Invalid rendering intent
    InvalidRenderingIntent(u32),
    /// Unsupported profile version
    UnsupportedVersion { major: u8, minor: u8 },
    /// Corrupted or invalid data
    CorruptedData(String),
    /// Unsupported feature
    Unsupported(String),
}

impl fmt::Display for IccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooSmall { expected, actual } => {
                write!(
                    f,
                    "Profile too small: expected {} bytes, got {}",
                    expected, actual
                )
            }
            Self::InvalidSignature(sig) => {
                write!(
                    f,
                    "Invalid profile signature: 0x{:08X} (expected 'acsp')",
                    sig
                )
            }
            Self::SizeMismatch {
                header_size,
                actual_size,
            } => {
                write!(
                    f,
                    "Size mismatch: header says {} bytes, data is {} bytes",
                    header_size, actual_size
                )
            }
            Self::TagOutOfBounds {
                tag,
                offset,
                size,
                profile_size,
            } => {
                write!(
                    f,
                    "Tag '{:08X}' out of bounds: offset {} + size {} > profile size {}",
                    tag, offset, size, profile_size
                )
            }
            Self::InvalidTagType { tag, type_sig } => {
                write!(f, "Invalid type '{:08X}' for tag '{:08X}'", type_sig, tag)
            }
            Self::MissingTag(tag) => {
                write!(f, "Required tag missing: '{:08X}'", tag)
            }
            Self::InvalidColorSpace(cs) => {
                write!(f, "Invalid color space: 0x{:08X}", cs)
            }
            Self::InvalidProfileClass(class) => {
                write!(f, "Invalid profile class: 0x{:08X}", class)
            }
            Self::InvalidRenderingIntent(intent) => {
                write!(f, "Invalid rendering intent: {}", intent)
            }
            Self::UnsupportedVersion { major, minor } => {
                write!(f, "Unsupported profile version: {}.{}", major, minor)
            }
            Self::CorruptedData(msg) => {
                write!(f, "Corrupted data: {}", msg)
            }
            Self::Unsupported(msg) => {
                write!(f, "Unsupported feature: {}", msg)
            }
        }
    }
}

impl std::error::Error for IccError {}
