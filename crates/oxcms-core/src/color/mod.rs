//! Color space types and conversions
//!
//! This module provides:
//! - CIE XYZ color space
//! - CIELAB (L*a*b*) color space
//! - XYB color space (JPEG XL perceptual)
//! - RGB primitives
//! - White point definitions

pub mod lab;
pub mod rgb;
pub mod white_point;
pub mod xyb;
pub mod xyz;

pub use lab::Lab;
pub use rgb::Rgb;
pub use white_point::{D50, D55, D60, D65, D75, DCI_P3, WhitePoint};
pub use xyb::{LinearRgb, Xyb, linear_rgb_to_xyb, srgb_to_xyb, xyb_to_linear_rgb, xyb_to_srgb};
pub use xyz::Xyz;
