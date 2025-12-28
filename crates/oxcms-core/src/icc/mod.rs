//! ICC Profile Parsing
//!
//! This module provides native ICC profile parsing according to ICC.1:2022.
//!
//! # Structure
//!
//! An ICC profile consists of:
//! 1. A 128-byte header
//! 2. A tag table listing all tags
//! 3. Tag data (may overlap)
//!
//! # Supported Profile Types
//!
//! - Matrix/TRC profiles (most RGB profiles)
//! - LUT-based profiles (CMYK, device links)
//! - Named color profiles (Pantone, etc.)
//!
//! # Example
//!
//! ```ignore
//! use oxcms_core::icc::IccProfile;
//!
//! let profile = IccProfile::parse(&bytes)?;
//! if profile.is_matrix_shaper() {
//!     // Use matrix/TRC for fast RGB transforms
//! }
//! ```

pub mod header;
pub mod tags;

mod error;
mod parser;
mod types;

pub use error::IccError;
pub use header::{ColorSpace, IccHeader, ProfileClass, RenderingIntent as IccRenderingIntent};
pub use parser::IccProfile;
pub use tags::{CurveData, ParametricCurveData, TagData};
pub use types::{DateTimeNumber, S15Fixed16, TagSignature, XyzNumber};
