//! SIMD-optimized matrix operations
//!
//! Matrix-vector multiplication is a core operation in color transforms.
//! This module provides optimized versions for different SIMD instruction sets.

use multiversion::multiversion;

/// Multiply a 3x3 matrix by a 3-element vector
///
/// This is the core operation for RGB↔XYZ conversions.
/// The matrix is stored in row-major order.
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn matrix_multiply_vec3(matrix: &[[f64; 3]; 3], vec: [f64; 3]) -> [f64; 3] {
    // Even with SIMD, f64 operations on 3-element vectors are tricky
    // because they don't align well with 128/256-bit registers.
    // For now, use scalar operations - the compiler will auto-vectorize
    // when processing batches.
    [
        matrix[0][0] * vec[0] + matrix[0][1] * vec[1] + matrix[0][2] * vec[2],
        matrix[1][0] * vec[0] + matrix[1][1] * vec[1] + matrix[1][2] * vec[2],
        matrix[2][0] * vec[0] + matrix[2][1] * vec[1] + matrix[2][2] * vec[2],
    ]
}

/// Multiply a 3x3 matrix by a batch of 3-element vectors
///
/// This is more amenable to SIMD optimization as we can process
/// multiple pixels at once.
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn matrix_multiply_vec3_batch(
    matrix: &[[f64; 3]; 3],
    input: &[[f64; 3]],
    output: &mut [[f64; 3]],
) {
    assert!(output.len() >= input.len());

    // Extract matrix elements for better register allocation
    let m00 = matrix[0][0];
    let m01 = matrix[0][1];
    let m02 = matrix[0][2];
    let m10 = matrix[1][0];
    let m11 = matrix[1][1];
    let m12 = matrix[1][2];
    let m20 = matrix[2][0];
    let m21 = matrix[2][1];
    let m22 = matrix[2][2];

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        let r = inp[0];
        let g = inp[1];
        let b = inp[2];

        out[0] = m00 * r + m01 * g + m02 * b;
        out[1] = m10 * r + m11 * g + m12 * b;
        out[2] = m20 * r + m21 * g + m22 * b;
    }
}

/// Multiply a 3x3 matrix by a batch of f32 vectors
///
/// f32 is more SIMD-friendly (4 per 128-bit, 8 per 256-bit register).
#[multiversion(targets("x86_64+avx2", "x86_64+sse4.1", "aarch64+neon",))]
pub fn matrix_multiply_vec3_batch_f32(
    matrix: &[[f32; 3]; 3],
    input: &[[f32; 3]],
    output: &mut [[f32; 3]],
) {
    assert!(output.len() >= input.len());

    let m00 = matrix[0][0];
    let m01 = matrix[0][1];
    let m02 = matrix[0][2];
    let m10 = matrix[1][0];
    let m11 = matrix[1][1];
    let m12 = matrix[1][2];
    let m20 = matrix[2][0];
    let m21 = matrix[2][1];
    let m22 = matrix[2][2];

    for (inp, out) in input.iter().zip(output.iter_mut()) {
        let r = inp[0];
        let g = inp[1];
        let b = inp[2];

        out[0] = m00 * r + m01 * g + m02 * b;
        out[1] = m10 * r + m11 * g + m12 * b;
        out[2] = m20 * r + m21 * g + m22 * b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiply_vec3() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let vec = [0.5, 0.3, 0.7];
        let result = matrix_multiply_vec3(&identity, vec);

        assert!((result[0] - 0.5).abs() < 1e-10);
        assert!((result[1] - 0.3).abs() < 1e-10);
        assert!((result[2] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_matrix_multiply_vec3_batch() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let input = [[0.5, 0.3, 0.7], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let mut output = [[0.0; 3]; 3];

        matrix_multiply_vec3_batch(&identity, &input, &mut output);

        for (inp, out) in input.iter().zip(output.iter()) {
            assert!((inp[0] - out[0]).abs() < 1e-10);
            assert!((inp[1] - out[1]).abs() < 1e-10);
            assert!((inp[2] - out[2]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_matrix_multiply_non_identity() {
        // sRGB to XYZ matrix (approximate)
        let srgb_to_xyz = [
            [0.4124564, 0.3575761, 0.1804375],
            [0.2126729, 0.7151522, 0.0721750],
            [0.0193339, 0.1191920, 0.9503041],
        ];

        // White (1,1,1) should give D65 white point approximately
        let white = [1.0, 1.0, 1.0];
        let xyz = matrix_multiply_vec3(&srgb_to_xyz, white);

        // D65 XYZ is approximately (0.95, 1.0, 1.09)
        assert!((xyz[0] - 0.95).abs() < 0.01);
        assert!((xyz[1] - 1.0).abs() < 0.01);
        assert!((xyz[2] - 1.09).abs() < 0.02);
    }
}
