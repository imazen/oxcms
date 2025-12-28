//! Interpolation functions for LUT evaluation
//!
//! This module provides:
//! - Linear interpolation (1D)
//! - Trilinear interpolation (3D LUT)
//! - Tetrahedral interpolation (3D LUT, more accurate)

/// Linear interpolation between two values
///
/// Returns a + t * (b - a) for t in [0, 1]
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Linear interpolation for 3-component vectors
#[inline]
pub fn lerp3(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

/// Bilinear interpolation in a 2D grid
///
/// # Arguments
/// * `c00`, `c10`, `c01`, `c11` - Corner values (cXY where X=column, Y=row)
/// * `tx`, `ty` - Interpolation parameters in [0, 1]
#[inline]
pub fn bilinear(c00: f64, c10: f64, c01: f64, c11: f64, tx: f64, ty: f64) -> f64 {
    let top = lerp(c00, c10, tx);
    let bottom = lerp(c01, c11, tx);
    lerp(top, bottom, ty)
}

/// Trilinear interpolation in a 3D grid
///
/// # Arguments
/// * `c` - Array of 8 corner values in order: [000, 100, 010, 110, 001, 101, 011, 111]
///         where the indices represent (x, y, z) positions
/// * `tx`, `ty`, `tz` - Interpolation parameters in [0, 1]
#[inline]
pub fn trilinear(c: [f64; 8], tx: f64, ty: f64, tz: f64) -> f64 {
    // Interpolate along x
    let c00 = lerp(c[0], c[1], tx);
    let c10 = lerp(c[2], c[3], tx);
    let c01 = lerp(c[4], c[5], tx);
    let c11 = lerp(c[6], c[7], tx);

    // Interpolate along y
    let c0 = lerp(c00, c10, ty);
    let c1 = lerp(c01, c11, ty);

    // Interpolate along z
    lerp(c0, c1, tz)
}

/// Trilinear interpolation for 3D color LUT
///
/// # Arguments
/// * `lut` - 3D LUT data, flattened in (r, g, b, channel) order
/// * `grid_size` - Number of grid points in each dimension
/// * `input` - Input RGB values in [0, 1]
///
/// # Returns
/// Interpolated RGB output
pub fn trilinear_interp(lut: &[f64], grid_size: usize, input: [f64; 3]) -> [f64; 3] {
    let max_idx = (grid_size - 1) as f64;

    // Scale input to grid coordinates
    let r = (input[0] * max_idx).clamp(0.0, max_idx);
    let g = (input[1] * max_idx).clamp(0.0, max_idx);
    let b = (input[2] * max_idx).clamp(0.0, max_idx);

    // Get integer grid positions
    let r0 = r.floor() as usize;
    let g0 = g.floor() as usize;
    let b0 = b.floor() as usize;

    let r1 = (r0 + 1).min(grid_size - 1);
    let g1 = (g0 + 1).min(grid_size - 1);
    let b1 = (b0 + 1).min(grid_size - 1);

    // Fractional parts
    let fr = r - r0 as f64;
    let fg = g - g0 as f64;
    let fb = b - b0 as f64;

    // Helper to get LUT value at grid position
    let idx = |r: usize, g: usize, b: usize, c: usize| -> f64 {
        let i = ((r * grid_size + g) * grid_size + b) * 3 + c;
        lut.get(i).copied().unwrap_or(0.0)
    };

    let mut output = [0.0; 3];
    for c in 0..3 {
        let corners = [
            idx(r0, g0, b0, c),
            idx(r1, g0, b0, c),
            idx(r0, g1, b0, c),
            idx(r1, g1, b0, c),
            idx(r0, g0, b1, c),
            idx(r1, g0, b1, c),
            idx(r0, g1, b1, c),
            idx(r1, g1, b1, c),
        ];
        output[c] = trilinear(corners, fr, fg, fb);
    }

    output
}

/// Tetrahedral interpolation for 3D color LUT
///
/// Tetrahedral interpolation divides each cube into 6 tetrahedra and
/// interpolates within the appropriate one. This is more accurate than
/// trilinear for color transformations.
///
/// # Arguments
/// * `lut` - 3D LUT data, flattened in (r, g, b, channel) order
/// * `grid_size` - Number of grid points in each dimension
/// * `input` - Input RGB values in [0, 1]
///
/// # Returns
/// Interpolated RGB output
pub fn tetrahedral_interp(lut: &[f64], grid_size: usize, input: [f64; 3]) -> [f64; 3] {
    let max_idx = (grid_size - 1) as f64;

    // Scale input to grid coordinates
    let r = (input[0] * max_idx).clamp(0.0, max_idx);
    let g = (input[1] * max_idx).clamp(0.0, max_idx);
    let b = (input[2] * max_idx).clamp(0.0, max_idx);

    // Get integer grid positions
    let r0 = r.floor() as usize;
    let g0 = g.floor() as usize;
    let b0 = b.floor() as usize;

    let r1 = (r0 + 1).min(grid_size - 1);
    let g1 = (g0 + 1).min(grid_size - 1);
    let b1 = (b0 + 1).min(grid_size - 1);

    // Fractional parts
    let fr = r - r0 as f64;
    let fg = g - g0 as f64;
    let fb = b - b0 as f64;

    // Helper to get LUT value at grid position
    let get = |r: usize, g: usize, b: usize| -> [f64; 3] {
        let base = ((r * grid_size + g) * grid_size + b) * 3;
        [
            lut.get(base).copied().unwrap_or(0.0),
            lut.get(base + 1).copied().unwrap_or(0.0),
            lut.get(base + 2).copied().unwrap_or(0.0),
        ]
    };

    // Get the 8 corner values
    let c000 = get(r0, g0, b0);
    let c100 = get(r1, g0, b0);
    let c010 = get(r0, g1, b0);
    let c110 = get(r1, g1, b0);
    let c001 = get(r0, g0, b1);
    let c101 = get(r1, g0, b1);
    let c011 = get(r0, g1, b1);
    let c111 = get(r1, g1, b1);

    // Determine which tetrahedron we're in and interpolate
    // There are 6 tetrahedra based on the ordering of fr, fg, fb
    let mut output = [0.0; 3];

    for c in 0..3 {
        output[c] = if fr > fg {
            if fg > fb {
                // fr > fg > fb: tetrahedron (000, 100, 110, 111)
                c000[c]
                    + fr * (c100[c] - c000[c])
                    + fg * (c110[c] - c100[c])
                    + fb * (c111[c] - c110[c])
            } else if fr > fb {
                // fr > fb > fg: tetrahedron (000, 100, 101, 111)
                c000[c]
                    + fr * (c100[c] - c000[c])
                    + fb * (c101[c] - c100[c])
                    + fg * (c111[c] - c101[c])
            } else {
                // fb > fr > fg: tetrahedron (000, 001, 101, 111)
                c000[c]
                    + fb * (c001[c] - c000[c])
                    + fr * (c101[c] - c001[c])
                    + fg * (c111[c] - c101[c])
            }
        } else if fg > fb {
            if fr > fb {
                // fg > fr > fb: tetrahedron (000, 010, 110, 111)
                c000[c]
                    + fg * (c010[c] - c000[c])
                    + fr * (c110[c] - c010[c])
                    + fb * (c111[c] - c110[c])
            } else {
                // fg > fb > fr: tetrahedron (000, 010, 011, 111)
                c000[c]
                    + fg * (c010[c] - c000[c])
                    + fb * (c011[c] - c010[c])
                    + fr * (c111[c] - c011[c])
            }
        } else {
            // fb > fg > fr: tetrahedron (000, 001, 011, 111)
            c000[c]
                + fb * (c001[c] - c000[c])
                + fg * (c011[c] - c001[c])
                + fr * (c111[c] - c011[c])
        };
    }

    output
}

/// Lookup in a 1D LUT with linear interpolation
///
/// # Arguments
/// * `lut` - 1D LUT values
/// * `input` - Input value in [0, 1]
///
/// # Returns
/// Interpolated output value
pub fn lut1d_interp(lut: &[f64], input: f64) -> f64 {
    if lut.is_empty() {
        return input;
    }
    if lut.len() == 1 {
        return lut[0];
    }

    let max_idx = (lut.len() - 1) as f64;
    let pos = (input * max_idx).clamp(0.0, max_idx);

    let i0 = pos.floor() as usize;
    let i1 = (i0 + 1).min(lut.len() - 1);
    let t = pos - i0 as f64;

    lerp(lut[i0], lut[i1], t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 1.0, 0.0) - 0.0).abs() < EPSILON);
        assert!((lerp(0.0, 1.0, 1.0) - 1.0).abs() < EPSILON);
        assert!((lerp(0.0, 1.0, 0.5) - 0.5).abs() < EPSILON);
        assert!((lerp(2.0, 4.0, 0.25) - 2.5).abs() < EPSILON);
    }

    #[test]
    fn test_bilinear() {
        // Identity at corners
        assert!((bilinear(0.0, 1.0, 2.0, 3.0, 0.0, 0.0) - 0.0).abs() < EPSILON);
        assert!((bilinear(0.0, 1.0, 2.0, 3.0, 1.0, 0.0) - 1.0).abs() < EPSILON);
        assert!((bilinear(0.0, 1.0, 2.0, 3.0, 0.0, 1.0) - 2.0).abs() < EPSILON);
        assert!((bilinear(0.0, 1.0, 2.0, 3.0, 1.0, 1.0) - 3.0).abs() < EPSILON);

        // Center should be average
        let center = bilinear(0.0, 1.0, 2.0, 3.0, 0.5, 0.5);
        assert!((center - 1.5).abs() < EPSILON);
    }

    #[test]
    fn test_trilinear_corners() {
        let corners = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];

        assert!((trilinear(corners, 0.0, 0.0, 0.0) - 0.0).abs() < EPSILON);
        assert!((trilinear(corners, 1.0, 0.0, 0.0) - 1.0).abs() < EPSILON);
        assert!((trilinear(corners, 0.0, 1.0, 0.0) - 2.0).abs() < EPSILON);
        assert!((trilinear(corners, 1.0, 1.0, 0.0) - 3.0).abs() < EPSILON);
        assert!((trilinear(corners, 0.0, 0.0, 1.0) - 4.0).abs() < EPSILON);
        assert!((trilinear(corners, 1.0, 1.0, 1.0) - 7.0).abs() < EPSILON);
    }

    #[test]
    fn test_identity_lut() {
        // Build an identity 3x3x3 LUT
        let grid_size = 3;
        let mut lut = vec![0.0; grid_size * grid_size * grid_size * 3];

        for r in 0..grid_size {
            for g in 0..grid_size {
                for b in 0..grid_size {
                    let idx = ((r * grid_size + g) * grid_size + b) * 3;
                    lut[idx] = r as f64 / (grid_size - 1) as f64;
                    lut[idx + 1] = g as f64 / (grid_size - 1) as f64;
                    lut[idx + 2] = b as f64 / (grid_size - 1) as f64;
                }
            }
        }

        // Test that it's identity
        let inputs = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
            [0.25, 0.5, 0.75],
        ];

        for input in inputs {
            let tri_output = trilinear_interp(&lut, grid_size, input);
            let tet_output = tetrahedral_interp(&lut, grid_size, input);

            for c in 0..3 {
                assert!(
                    (tri_output[c] - input[c]).abs() < 1e-9,
                    "Trilinear identity failed: {:?} -> {:?}",
                    input,
                    tri_output
                );
                assert!(
                    (tet_output[c] - input[c]).abs() < 1e-9,
                    "Tetrahedral identity failed: {:?} -> {:?}",
                    input,
                    tet_output
                );
            }
        }
    }

    #[test]
    fn test_lut1d() {
        let lut = vec![0.0, 0.5, 1.0];

        assert!((lut1d_interp(&lut, 0.0) - 0.0).abs() < EPSILON);
        assert!((lut1d_interp(&lut, 0.5) - 0.5).abs() < EPSILON);
        assert!((lut1d_interp(&lut, 1.0) - 1.0).abs() < EPSILON);
        assert!((lut1d_interp(&lut, 0.25) - 0.25).abs() < EPSILON);
    }
}
