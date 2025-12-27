//! # cms-tests
//!
//! Cross-CMS parity testing framework for oxcms.
//!
//! This crate provides:
//! - Parity tests comparing oxcms output against reference implementations
//! - Accuracy measurements using deltaE2000
//! - Math difference documentation
//! - Test corpus management
//!
//! ## Reference Implementations
//!
//! - **lcms2**: Industry standard, full ICC support
//! - **moxcms**: Pure Rust reference for performance baseline
//! - **skcms**: Chrome's CMS (via test corpus, no direct comparison)
//! - **qcms**: Firefox's CMS (pure Rust)
//!
//! ## Test Categories
//!
//! 1. **Profile Parsing**: Validate ICC profile parsing
//! 2. **RGB Transforms**: sRGB, Display P3, Adobe RGB
//! 3. **CMYK Transforms**: FOGRA, GRACoL
//! 4. **Lab/XYZ Transforms**: Color space conversions
//! 5. **Edge Cases**: Malformed profiles, boundary values

pub mod accuracy;
pub mod corpus;
pub mod parity;
pub mod patterns;
pub mod reference;

pub use accuracy::{DeltaEStats, compare_rgb_buffers, delta_e_2000};
pub use parity::ParityTest;
