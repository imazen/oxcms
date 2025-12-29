//! Mathematical operations for color management
//!
//! This module provides foundational math operations used throughout oxcms:
//! - 3x3 matrix operations for RGB↔XYZ transforms
//! - Gamma and transfer function evaluation
//! - Chromatic adaptation (Bradford)
//! - Interpolation for LUT evaluation

pub mod chromatic_adaptation;
pub mod gamma;
pub mod interpolation;
pub mod matrix;

pub use chromatic_adaptation::{
    ChromaticAdaptationMethod, adapt_xyz, adaptation_matrix, bradford_matrix,
};
pub use gamma::{
    ParametricCurve, ParametricCurveType, parametric_curve_eval, srgb_gamma_decode,
    srgb_gamma_encode,
};
pub use interpolation::{lerp, tetrahedral_interp, trilinear_interp};
pub use matrix::Matrix3x3;
