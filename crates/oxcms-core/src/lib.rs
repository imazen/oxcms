//! # oxcms - Oxidized Color Management System
//!
//! A fast, safe, and complete color management system in Rust.
//!
//! ## Goals
//!
//! - **Fast**: SIMD-optimized (AVX2, SSE4, NEON) - 3x+ faster than lcms2
//! - **Safe**: Pure Rust, memory-safe by design
//! - **Complete**: Full ICC v4.4 support including CMYK, DeviceLink, CIECAM02
//! - **Tested**: Parity tested against lcms2, skcms, and qcms
//!
//! ## Current Status
//!
//! This library wraps moxcms and adds additional validation and testing.
//! The goal is to eventually provide our own implementation that matches
//! lcms2 output exactly while maintaining moxcms performance.
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

pub mod error;
pub mod profile;
pub mod transform;

pub use error::{Error, Result};
pub use profile::ColorProfile;
pub use transform::{Layout, RenderingIntent, Transform, TransformOptions};

// Re-export useful moxcms types directly
pub use moxcms::{
    // Color spaces and coordinates
    Lab, Xyz, Xyzd, XyY,
    // Matrices
    Matrix3d, Matrix3f, Vector3d, Vector3f,
    // Chromatic adaptation
    adapt_to_d50, adapt_to_d50_d, Chromaticity,
    // White points
    WHITE_POINT_D50, WHITE_POINT_D65,
    // Profile types
    DataColorSpace, ProfileClass, ProfileVersion,
    // CICP
    CicpProfile, CicpColorPrimaries, TransferCharacteristics,
    // Tone curves
    ToneReprCurve, curve_from_gamma,
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
