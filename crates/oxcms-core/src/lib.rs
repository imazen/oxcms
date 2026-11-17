//! # oxcms - Oxidized Color Management System
//!
//! A Rust color-management wrapper with native color and ICC modules.
//!
//! The top-level profile/transform API delegates to vendored moxcms 0.8.0.
//! Native curve code is also present; it is not a complete replacement for
//! that backend. Cross-CMS parity and ICC conformance are separate checks.
//! Known workspace failures and the comparison with current upstream moxcms
//! are documented in the repository's CHANGELOG and docs/ICC_DEVIATIONS.md.
//!
//! ## Quick Start
//!
//! ```no_run
//! use oxcms_core::{ColorProfile, Layout, TransformOptions};
//!
//! // Create sRGB profile
//! let srgb = ColorProfile::new_srgb();
//!
//! // Create Display P3 profile
//! let p3 = ColorProfile::new_display_p3();
//!
//! // Create transform
//! let transform = srgb.create_transform_8bit(
//!     Layout::Rgb,
//!     &p3,
//!     Layout::Rgb,
//!     TransformOptions::default(),
//! ).unwrap();
//!
//! // Transform pixels
//! let src = [255u8, 128, 64];
//! let mut dst = [0u8; 3];
//! transform.transform(&src, &mut dst).unwrap();
//! ```
//!
//! ## AI-Generated Code Notice
//!
//! This crate was developed with assistance from Claude (Anthropic).
//! Not all code has been manually reviewed. Validate independently before production use.

// Native oxcms modules (Phase 2 implementation)
pub mod color;
pub mod icc;
pub mod math;
pub mod pipeline;
pub mod simd;
pub mod types;

// Legacy moxcms wrapper modules (Phase 1)
pub mod error;
pub mod profile;
pub mod transform;

pub use error::{Error, Result};
pub use profile::ColorProfile;
pub use transform::{Layout, Transform, TransformOptions};
pub use types::{ColorSpace, Matrix3x3, ProfileClass, ProfileVersion, RenderingIntent, XyzColor};

// Re-export useful moxcms types directly
// Note: ProfileClass, ProfileVersion, RenderingIntent are in types module (own wrappers)
pub use moxcms::{
    Chromaticity,
    CicpColorPrimaries,
    // CICP
    CicpProfile,
    // Profile types
    DataColorSpace,
    // Color spaces and coordinates
    Lab,
    // Matrices
    Matrix3d,
    Matrix3f,
    // Tone curves
    ToneReprCurve,
    TransferCharacteristics,
    Vector3d,
    Vector3f,
    // White points
    WHITE_POINT_D50,
    WHITE_POINT_D65,
    XyY,
    Xyz,
    Xyzd,
    // Chromatic adaptation
    adapt_to_d50,
    adapt_to_d50_d,
    curve_from_gamma,
};

/// Version of oxcms
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if two profiles would produce identical transforms
pub fn profiles_equivalent(a: &ColorProfile, b: &ColorProfile) -> bool {
    // Check basic properties
    if a.inner().color_space != b.inner().color_space {
        return false;
    }
    if a.inner().pcs != b.inner().pcs {
        return false;
    }

    // For matrix-shaper profiles, check colorants and TRCs
    if a.is_matrix_shaper() && b.is_matrix_shaper() {
        let eps = 1e-6;
        let a_mat = a.inner().colorant_matrix();
        let b_mat = b.inner().colorant_matrix();

        for i in 0..3 {
            for j in 0..3 {
                if (a_mat.v[i][j] - b_mat.v[i][j]).abs() > eps {
                    return false;
                }
            }
        }

        // Would need to compare TRCs as well for full equivalence
        return true;
    }

    false
}
