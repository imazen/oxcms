//! Error types for oxcms

use thiserror::Error;

/// Result type for oxcms operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in oxcms operations
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Failed to parse ICC profile
    #[error("Profile parse error: {0}")]
    ProfileParse(String),

    /// Invalid ICC profile structure
    #[error("Invalid profile: {0}")]
    InvalidProfile(String),

    /// Unsupported profile version
    #[error("Unsupported profile version: {0}")]
    UnsupportedVersion(String),

    /// Unsupported color space
    #[error("Unsupported color space: {0}")]
    UnsupportedColorSpace(String),

    /// Transform creation failed
    #[error("Transform error: {0}")]
    Transform(String),

    /// Buffer size mismatch
    #[error("Buffer size mismatch: expected {expected}, got {actual}")]
    BufferSize { expected: usize, actual: usize },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
