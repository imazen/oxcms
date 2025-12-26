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
//! This is a work-in-progress fork/rewrite based on moxcms.
//! See `tracking/TEST_STATUS.md` for current test coverage.
//!
//! ## AI-Generated Code Notice
//!
//! This crate was developed with assistance from Claude (Anthropic).
//! Not all code has been manually reviewed. Validate independently before production use.

#![forbid(unsafe_code)] // Enforced until we add audited SIMD

pub mod error;
pub mod profile;
pub mod transform;

pub use error::{Error, Result};
pub use profile::ColorProfile;
pub use transform::Transform;
