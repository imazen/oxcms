//! SIMD-Optimized Color Operations
//!
//! This module provides SIMD-accelerated versions of performance-critical
//! color operations using the `multiversion` crate for automatic CPU dispatch.
//!
//! Supported instruction sets:
//! - x86-64: SSE4.1, AVX2
//! - ARM64: NEON
//!
//! # Usage
//!
//! All functions automatically dispatch to the best available implementation
//! at runtime. The scalar fallback is always available.

mod batch;
mod gamma;
mod matrix;

pub use batch::{
    clamp_rgb_batch, f64_to_rgb8_batch, rgb8_to_f64_batch, transform_rgb16_batch,
    transform_rgb8_batch,
};
pub use gamma::{
    apply_gamma_batch, apply_lut1d_batch, apply_srgb_decode_batch, apply_srgb_encode_batch,
};
pub use matrix::{matrix_multiply_vec3, matrix_multiply_vec3_batch};

/// Check if AVX2 is available at runtime
#[cfg(target_arch = "x86_64")]
pub fn has_avx2() -> bool {
    is_x86_feature_detected!("avx2")
}

/// Check if SSE4.1 is available at runtime
#[cfg(target_arch = "x86_64")]
pub fn has_sse41() -> bool {
    is_x86_feature_detected!("sse4.1")
}

/// Check if NEON is available (always true on aarch64)
#[cfg(target_arch = "aarch64")]
pub fn has_neon() -> bool {
    true
}

/// Get a description of the active SIMD features
pub fn active_features() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            "AVX2"
        } else if is_x86_feature_detected!("sse4.1") {
            "SSE4.1"
        } else {
            "scalar"
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        "NEON"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "scalar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_features() {
        let features = active_features();
        println!("Active SIMD features: {}", features);
        assert!(!features.is_empty());
    }
}
