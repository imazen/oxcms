//! SIMD-optimized batch color transforms
//!
//! These functions process entire buffers of pixels efficiently.

use multiversion::multiversion;

/// Transform a buffer of RGB8 pixels
///
/// Combines u8→f64 conversion, transform, and f64→u8 conversion.
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn transform_rgb8_batch<F>(src: &[u8], dst: &mut [u8], transform_fn: F)
where
    F: Fn([f64; 3]) -> [f64; 3],
{
    assert!(src.len() % 3 == 0);
    assert!(dst.len() >= src.len());

    for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(3)) {
        let rgb = [
            src_chunk[0] as f64 / 255.0,
            src_chunk[1] as f64 / 255.0,
            src_chunk[2] as f64 / 255.0,
        ];

        let result = transform_fn(rgb);

        dst_chunk[0] = (result[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        dst_chunk[1] = (result[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        dst_chunk[2] = (result[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    }
}

/// Transform a buffer of RGB16 pixels
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn transform_rgb16_batch<F>(src: &[u16], dst: &mut [u16], transform_fn: F)
where
    F: Fn([f64; 3]) -> [f64; 3],
{
    assert!(src.len() % 3 == 0);
    assert!(dst.len() >= src.len());

    for (src_chunk, dst_chunk) in src.chunks_exact(3).zip(dst.chunks_exact_mut(3)) {
        let rgb = [
            src_chunk[0] as f64 / 65535.0,
            src_chunk[1] as f64 / 65535.0,
            src_chunk[2] as f64 / 65535.0,
        ];

        let result = transform_fn(rgb);

        dst_chunk[0] = (result[0].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
        dst_chunk[1] = (result[1].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
        dst_chunk[2] = (result[2].clamp(0.0, 1.0) * 65535.0 + 0.5) as u16;
    }
}

/// Convert u8 RGB to normalized f64 RGB
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn rgb8_to_f64_batch(src: &[u8], dst: &mut [[f64; 3]]) {
    assert!(src.len() % 3 == 0);
    assert!(dst.len() >= src.len() / 3);

    for (src_chunk, out) in src.chunks_exact(3).zip(dst.iter_mut()) {
        out[0] = src_chunk[0] as f64 / 255.0;
        out[1] = src_chunk[1] as f64 / 255.0;
        out[2] = src_chunk[2] as f64 / 255.0;
    }
}

/// Convert normalized f64 RGB to u8 RGB
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn f64_to_rgb8_batch(src: &[[f64; 3]], dst: &mut [u8]) {
    assert!(dst.len() >= src.len() * 3);

    for (inp, dst_chunk) in src.iter().zip(dst.chunks_exact_mut(3)) {
        dst_chunk[0] = (inp[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        dst_chunk[1] = (inp[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        dst_chunk[2] = (inp[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    }
}

/// Apply clamping to a batch of RGB values
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn clamp_rgb_batch(data: &mut [[f64; 3]]) {
    for rgb in data.iter_mut() {
        rgb[0] = rgb[0].clamp(0.0, 1.0);
        rgb[1] = rgb[1].clamp(0.0, 1.0);
        rgb[2] = rgb[2].clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_rgb8_batch() {
        let src = [255u8, 128, 64, 0, 255, 128];
        let mut dst = [0u8; 6];

        // Identity transform
        transform_rgb8_batch(&src, &mut dst, |rgb| rgb);

        assert_eq!(dst[0], 255);
        assert_eq!(dst[1], 128);
        assert_eq!(dst[2], 64);
        assert_eq!(dst[3], 0);
        assert_eq!(dst[4], 255);
        assert_eq!(dst[5], 128);
    }

    #[test]
    fn test_rgb8_f64_roundtrip() {
        let src = [0u8, 128, 255, 64, 192, 32];
        let mut f64_buf = [[0.0; 3]; 2];
        let mut dst = [0u8; 6];

        rgb8_to_f64_batch(&src, &mut f64_buf);
        f64_to_rgb8_batch(&f64_buf, &mut dst);

        assert_eq!(src, dst);
    }

    #[test]
    fn test_clamp_rgb_batch() {
        let mut data = [[1.5, -0.5, 0.5], [0.0, 1.0, 2.0]];

        clamp_rgb_batch(&mut data);

        assert!((data[0][0] - 1.0).abs() < 1e-10);
        assert!((data[0][1] - 0.0).abs() < 1e-10);
        assert!((data[0][2] - 0.5).abs() < 1e-10);
        assert!((data[1][2] - 1.0).abs() < 1e-10);
    }
}
