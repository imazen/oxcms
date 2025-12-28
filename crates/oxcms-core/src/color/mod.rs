//! Color space types and conversions
//!
//! This module provides:
//! - CIE XYZ color space
//! - CIELAB (L*a*b*) color space
//! - RGB primitives
//! - White point definitions

pub mod lab;
pub mod rgb;
pub mod white_point;
pub mod xyz;

pub use lab::Lab;
pub use rgb::Rgb;
pub use white_point::{WhitePoint, D50, D55, D60, D65, D75, DCI_P3};
pub use xyz::Xyz;
