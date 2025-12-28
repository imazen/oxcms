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

pub use chromatic_adaptation::{adapt_xyz, adaptation_matrix, bradford_matrix, ChromaticAdaptationMethod};
pub use gamma::{
    parametric_curve_eval, srgb_gamma_decode, srgb_gamma_encode, ParametricCurve,
    ParametricCurveType,
};
pub use interpolation::{lerp, tetrahedral_interp, trilinear_interp};
pub use matrix::Matrix3x3;
