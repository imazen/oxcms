//! LUT-Based Transform Pipeline
//!
//! LUT (Look-Up Table) based transforms are used for:
//! - CMYK profiles
//! - DeviceLink profiles
//! - Complex multi-dimensional transformations
//!
//! The LUT pipeline can include:
//! - A curves (input)
//! - CLUT (3D lookup table)
//! - M curves (middle)
//! - Matrix (optional)
//! - B curves (output)

use crate::icc::IccError;
use crate::icc::tags::{CurveSegment, Lut8Data, Lut16Data, LutAToBData, LutBToAData, TagData};
use crate::math::{tetrahedral_interp, trilinear_interp};

/// A LUT-based transform pipeline
#[derive(Debug, Clone)]
pub struct LutPipeline {
    /// Number of input channels
    pub input_channels: usize,
    /// Number of output channels
    pub output_channels: usize,
    /// Input curves
    input_curves: Vec<LutCurve>,
    /// CLUT data (normalized to f64)
    clut: Option<ClutData>,
    /// Output curves
    output_curves: Vec<LutCurve>,
    /// Whether to use tetrahedral interpolation (vs trilinear)
    use_tetrahedral: bool,
}

/// Normalized CLUT data
#[derive(Debug, Clone)]
pub struct ClutData {
    /// Grid points per dimension
    pub grid_points: Vec<usize>,
    /// Number of output channels
    pub output_channels: usize,
    /// CLUT data (normalized 0.0-1.0)
    pub data: Vec<f64>,
}

/// A single LUT curve
#[derive(Debug, Clone)]
pub enum LutCurve {
    /// Identity curve
    Identity,
    /// Gamma curve
    Gamma(f64),
    /// Lookup table (normalized)
    Table(Vec<f64>),
}

impl Default for LutCurve {
    fn default() -> Self {
        LutCurve::Identity
    }
}

impl LutCurve {
    /// Create from 8-bit table
    pub fn from_u8_table(table: &[u8]) -> Self {
        if table.is_empty() {
            return LutCurve::Identity;
        }
        let normalized: Vec<f64> = table.iter().map(|&v| v as f64 / 255.0).collect();
        LutCurve::Table(normalized)
    }

    /// Create from 16-bit table
    pub fn from_u16_table(table: &[u16]) -> Self {
        if table.is_empty() {
            return LutCurve::Identity;
        }
        let normalized: Vec<f64> = table.iter().map(|&v| v as f64 / 65535.0).collect();
        LutCurve::Table(normalized)
    }

    /// Create from curve segment
    pub fn from_segment(segment: &CurveSegment) -> Self {
        match segment {
            CurveSegment::Identity => LutCurve::Identity,
            CurveSegment::Table(table) => {
                if table.is_empty() {
                    LutCurve::Identity
                } else {
                    LutCurve::Table(table.clone())
                }
            }
            CurveSegment::Parametric { curve_type, params } => {
                if *curve_type == 0 && !params.is_empty() {
                    // Simple gamma
                    LutCurve::Gamma(params[0])
                } else {
                    // For other types, build a lookup table
                    let mut table = Vec::with_capacity(256);
                    for i in 0..256 {
                        let x = i as f64 / 255.0;
                        let y = eval_parametric(*curve_type, params, x);
                        table.push(y);
                    }
                    LutCurve::Table(table)
                }
            }
        }
    }

    /// Evaluate the curve at input x (0.0-1.0)
    pub fn eval(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        match self {
            LutCurve::Identity => x,
            LutCurve::Gamma(g) => x.powf(*g),
            LutCurve::Table(table) => {
                if table.is_empty() {
                    return x;
                }
                if table.len() == 1 {
                    return table[0];
                }

                let pos = x * (table.len() - 1) as f64;
                let idx = pos.floor() as usize;
                let frac = pos - idx as f64;

                if idx >= table.len() - 1 {
                    return table[table.len() - 1];
                }

                table[idx] + frac * (table[idx + 1] - table[idx])
            }
        }
    }
}

/// Evaluate parametric curve
fn eval_parametric(curve_type: u16, params: &[f64], x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    let g = params.first().copied().unwrap_or(1.0);

    match curve_type {
        0 => {
            // Y = X^g
            x.powf(g)
        }
        1 => {
            // Y = (aX + b)^g  if X >= -b/a, else 0
            let a = params.get(1).copied().unwrap_or(1.0);
            let b = params.get(2).copied().unwrap_or(0.0);
            let threshold = if a.abs() > 1e-10 { -b / a } else { 0.0 };
            if x >= threshold {
                (a * x + b).max(0.0).powf(g)
            } else {
                0.0
            }
        }
        2 => {
            // Y = (aX + b)^g + c  if X >= -b/a, else c
            let a = params.get(1).copied().unwrap_or(1.0);
            let b = params.get(2).copied().unwrap_or(0.0);
            let c = params.get(3).copied().unwrap_or(0.0);
            let threshold = if a.abs() > 1e-10 { -b / a } else { 0.0 };
            if x >= threshold {
                (a * x + b).max(0.0).powf(g) + c
            } else {
                c
            }
        }
        3 => {
            // Y = (aX + b)^g  if X >= d, else cX
            let a = params.get(1).copied().unwrap_or(1.0);
            let b = params.get(2).copied().unwrap_or(0.0);
            let c = params.get(3).copied().unwrap_or(0.0);
            let d = params.get(4).copied().unwrap_or(0.0);
            if x >= d {
                (a * x + b).max(0.0).powf(g)
            } else {
                c * x
            }
        }
        4 => {
            // Y = (aX + b)^g + e  if X >= d, else cX + f
            let a = params.get(1).copied().unwrap_or(1.0);
            let b = params.get(2).copied().unwrap_or(0.0);
            let c = params.get(3).copied().unwrap_or(0.0);
            let d = params.get(4).copied().unwrap_or(0.0);
            let e = params.get(5).copied().unwrap_or(0.0);
            let f = params.get(6).copied().unwrap_or(0.0);
            if x >= d {
                (a * x + b).max(0.0).powf(g) + e
            } else {
                c * x + f
            }
        }
        _ => x,
    }
}

impl LutPipeline {
    /// Create an identity LUT pipeline (pass-through, no transformation)
    ///
    /// Useful for testing and as a fallback.
    pub fn identity(input_channels: usize, output_channels: usize) -> Self {
        Self {
            input_channels,
            output_channels,
            input_curves: vec![LutCurve::Identity; input_channels],
            clut: None,
            output_curves: vec![LutCurve::Identity; output_channels],
            use_tetrahedral: true,
        }
    }

    /// Create a LUT pipeline from a parsed TagData
    ///
    /// Supports Lut8, Lut16, LutAToB, and LutBToA tag types.
    pub fn from_tag_data(tag: &TagData) -> Result<Self, IccError> {
        match tag {
            TagData::Lut8(lut) => Ok(Self::from_lut8(lut)),
            TagData::Lut16(lut) => Ok(Self::from_lut16(lut)),
            TagData::LutAToB(lut) => Ok(Self::from_lut_atob(lut)),
            TagData::LutBToA(lut) => Ok(Self::from_lut_btoa(lut)),
            _ => Err(IccError::Unsupported("Tag is not a LUT type".to_string())),
        }
    }

    /// Create a LUT pipeline from Lut8 data
    pub fn from_lut8(lut: &Lut8Data) -> Self {
        // Parse input curves
        let input_curves: Vec<LutCurve> = lut
            .input_curves
            .iter()
            .map(|c| LutCurve::from_u8_table(c))
            .collect();

        // Parse CLUT
        let grid = lut.grid_points as usize;
        let grid_points = vec![grid; lut.input_channels as usize];

        let clut_data: Vec<f64> = lut.clut.iter().map(|&v| v as f64 / 255.0).collect();

        let clut = Some(ClutData {
            grid_points,
            output_channels: lut.output_channels as usize,
            data: clut_data,
        });

        // Parse output curves
        let output_curves: Vec<LutCurve> = lut
            .output_curves
            .iter()
            .map(|c| LutCurve::from_u8_table(c))
            .collect();

        Self {
            input_channels: lut.input_channels as usize,
            output_channels: lut.output_channels as usize,
            input_curves,
            clut,
            output_curves,
            use_tetrahedral: true,
        }
    }

    /// Create a LUT pipeline from Lut16 data
    pub fn from_lut16(lut: &Lut16Data) -> Self {
        // Parse input curves
        let input_curves: Vec<LutCurve> = lut
            .input_curves
            .iter()
            .map(|c| LutCurve::from_u16_table(c))
            .collect();

        // Parse CLUT
        let grid = lut.grid_points as usize;
        let grid_points = vec![grid; lut.input_channels as usize];

        let clut_data: Vec<f64> = lut.clut.iter().map(|&v| v as f64 / 65535.0).collect();

        let clut = Some(ClutData {
            grid_points,
            output_channels: lut.output_channels as usize,
            data: clut_data,
        });

        // Parse output curves
        let output_curves: Vec<LutCurve> = lut
            .output_curves
            .iter()
            .map(|c| LutCurve::from_u16_table(c))
            .collect();

        Self {
            input_channels: lut.input_channels as usize,
            output_channels: lut.output_channels as usize,
            input_curves,
            clut,
            output_curves,
            use_tetrahedral: true,
        }
    }

    /// Create a LUT pipeline from lutAToB data
    pub fn from_lut_atob(lut: &LutAToBData) -> Self {
        // A curves are input curves
        let input_curves = lut
            .a_curves
            .as_ref()
            .map(|curves| curves.iter().map(LutCurve::from_segment).collect())
            .unwrap_or_else(|| vec![LutCurve::Identity; lut.input_channels as usize]);

        // Parse CLUT if present
        let clut = lut.clut.as_ref().map(|c| ClutData {
            grid_points: c.grid_points.iter().map(|&g| g as usize).collect(),
            output_channels: c.output_channels as usize,
            data: c.data.clone(),
        });

        // B curves are output curves (for A2B)
        let output_curves = lut
            .b_curves
            .as_ref()
            .map(|curves| curves.iter().map(LutCurve::from_segment).collect())
            .unwrap_or_else(|| vec![LutCurve::Identity; lut.output_channels as usize]);

        Self {
            input_channels: lut.input_channels as usize,
            output_channels: lut.output_channels as usize,
            input_curves,
            clut,
            output_curves,
            use_tetrahedral: true,
        }
    }

    /// Create a LUT pipeline from lutBToA data
    pub fn from_lut_btoa(lut: &LutBToAData) -> Self {
        // B curves are input curves (for B2A)
        let input_curves = lut
            .b_curves
            .as_ref()
            .map(|curves| curves.iter().map(LutCurve::from_segment).collect())
            .unwrap_or_else(|| vec![LutCurve::Identity; lut.input_channels as usize]);

        // Parse CLUT if present
        let clut = lut.clut.as_ref().map(|c| ClutData {
            grid_points: c.grid_points.iter().map(|&g| g as usize).collect(),
            output_channels: c.output_channels as usize,
            data: c.data.clone(),
        });

        // A curves are output curves (for B2A)
        let output_curves = lut
            .a_curves
            .as_ref()
            .map(|curves| curves.iter().map(LutCurve::from_segment).collect())
            .unwrap_or_else(|| vec![LutCurve::Identity; lut.output_channels as usize]);

        Self {
            input_channels: lut.input_channels as usize,
            output_channels: lut.output_channels as usize,
            input_curves,
            clut,
            output_curves,
            use_tetrahedral: true,
        }
    }

    /// Transform input values through the LUT pipeline
    pub fn transform(&self, input: &[f64]) -> Vec<f64> {
        // Apply input curves
        let mut values: Vec<f64> = input
            .iter()
            .zip(self.input_curves.iter().cycle())
            .map(|(&x, curve)| curve.eval(x))
            .collect();

        // Apply CLUT
        if let Some(ref clut) = self.clut {
            values = self.apply_clut(&values, clut);
        }

        // Apply output curves
        values
            .iter()
            .zip(self.output_curves.iter().cycle())
            .map(|(&x, curve)| curve.eval(x))
            .collect()
    }

    /// Apply CLUT interpolation
    fn apply_clut(&self, input: &[f64], clut: &ClutData) -> Vec<f64> {
        // For 3-channel input, use optimized interpolation
        if input.len() == 3 && clut.grid_points.len() >= 3 {
            let grid_size = clut.grid_points[0];
            if self.use_tetrahedral {
                let result =
                    tetrahedral_interp(&clut.data, grid_size, [input[0], input[1], input[2]]);
                return result.to_vec();
            } else {
                let result =
                    trilinear_interp(&clut.data, grid_size, [input[0], input[1], input[2]]);
                return result.to_vec();
            }
        }

        // For 4-channel input (CMYK), use quadrilinear interpolation
        if input.len() == 4 && clut.grid_points.len() >= 4 {
            return self.apply_clut_4d(input, clut);
        }

        // For other dimensions, fall back to nearest-neighbor
        self.apply_clut_generic(input, clut)
    }

    /// Apply 4D CLUT interpolation (for CMYK)
    fn apply_clut_4d(&self, input: &[f64], clut: &ClutData) -> Vec<f64> {
        // Quadrilinear interpolation for 4D CLUT
        let g0 = clut.grid_points[0];
        let g1 = clut.grid_points[1];
        let g2 = clut.grid_points[2];
        let g3 = clut.grid_points[3];
        let out_ch = clut.output_channels;

        // Calculate positions and fractions for each dimension
        let p0 = (input[0].clamp(0.0, 1.0) * (g0 - 1) as f64).min((g0 - 1) as f64);
        let p1 = (input[1].clamp(0.0, 1.0) * (g1 - 1) as f64).min((g1 - 1) as f64);
        let p2 = (input[2].clamp(0.0, 1.0) * (g2 - 1) as f64).min((g2 - 1) as f64);
        let p3 = (input[3].clamp(0.0, 1.0) * (g3 - 1) as f64).min((g3 - 1) as f64);

        let i0 = p0.floor() as usize;
        let i1 = p1.floor() as usize;
        let i2 = p2.floor() as usize;
        let i3 = p3.floor() as usize;

        let f0 = p0 - i0 as f64;
        let f1 = p1 - i1 as f64;
        let f2 = p2 - i2 as f64;
        let f3 = p3 - i3 as f64;

        // Clamp indices
        let i0_1 = (i0 + 1).min(g0 - 1);
        let i1_1 = (i1 + 1).min(g1 - 1);
        let i2_1 = (i2 + 1).min(g2 - 1);
        let i3_1 = (i3 + 1).min(g3 - 1);

        // Calculate strides
        let s3 = out_ch;
        let s2 = s3 * g3;
        let s1 = s2 * g2;
        let s0 = s1 * g1;

        // Index function
        let idx = |c, m, y, k| c * s0 + m * s1 + y * s2 + k * s3;

        // Interpolate in all 4 dimensions
        let mut output = vec![0.0f64; out_ch];

        for ch in 0..out_ch {
            // 16 corner values
            let v0000 = clut
                .data
                .get(idx(i0, i1, i2, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0001 = clut
                .data
                .get(idx(i0, i1, i2, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0010 = clut
                .data
                .get(idx(i0, i1, i2_1, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0011 = clut
                .data
                .get(idx(i0, i1, i2_1, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0100 = clut
                .data
                .get(idx(i0, i1_1, i2, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0101 = clut
                .data
                .get(idx(i0, i1_1, i2, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0110 = clut
                .data
                .get(idx(i0, i1_1, i2_1, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v0111 = clut
                .data
                .get(idx(i0, i1_1, i2_1, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1000 = clut
                .data
                .get(idx(i0_1, i1, i2, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1001 = clut
                .data
                .get(idx(i0_1, i1, i2, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1010 = clut
                .data
                .get(idx(i0_1, i1, i2_1, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1011 = clut
                .data
                .get(idx(i0_1, i1, i2_1, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1100 = clut
                .data
                .get(idx(i0_1, i1_1, i2, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1101 = clut
                .data
                .get(idx(i0_1, i1_1, i2, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1110 = clut
                .data
                .get(idx(i0_1, i1_1, i2_1, i3) + ch)
                .copied()
                .unwrap_or(0.0);
            let v1111 = clut
                .data
                .get(idx(i0_1, i1_1, i2_1, i3_1) + ch)
                .copied()
                .unwrap_or(0.0);

            // Interpolate along dimension 3 (K)
            let v000 = v0000 + f3 * (v0001 - v0000);
            let v001 = v0010 + f3 * (v0011 - v0010);
            let v010 = v0100 + f3 * (v0101 - v0100);
            let v011 = v0110 + f3 * (v0111 - v0110);
            let v100 = v1000 + f3 * (v1001 - v1000);
            let v101 = v1010 + f3 * (v1011 - v1010);
            let v110 = v1100 + f3 * (v1101 - v1100);
            let v111 = v1110 + f3 * (v1111 - v1110);

            // Interpolate along dimension 2 (Y)
            let v00 = v000 + f2 * (v001 - v000);
            let v01 = v010 + f2 * (v011 - v010);
            let v10 = v100 + f2 * (v101 - v100);
            let v11 = v110 + f2 * (v111 - v110);

            // Interpolate along dimension 1 (M)
            let v0 = v00 + f1 * (v01 - v00);
            let v1 = v10 + f1 * (v11 - v10);

            // Interpolate along dimension 0 (C)
            output[ch] = v0 + f0 * (v1 - v0);
        }

        output
    }

    /// Generic CLUT interpolation for arbitrary dimensions
    fn apply_clut_generic(&self, input: &[f64], clut: &ClutData) -> Vec<f64> {
        // Simplified: just find nearest grid point
        let mut idx = 0usize;
        let mut stride = 1usize;

        for (i, &x) in input.iter().enumerate().rev() {
            if i >= clut.grid_points.len() {
                continue;
            }
            let grid = clut.grid_points[i];
            let pos = (x * (grid - 1) as f64).round() as usize;
            let pos = pos.min(grid - 1);
            idx += pos * stride;
            stride *= grid;
        }

        // Extract output values
        let base = idx * clut.output_channels;
        let mut output = Vec::with_capacity(clut.output_channels);
        for i in 0..clut.output_channels {
            output.push(clut.data.get(base + i).copied().unwrap_or(0.0));
        }
        output
    }

    /// Transform 3-channel input to 3-channel output (common case)
    pub fn transform_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        let result = self.transform(&rgb);
        [
            result.first().copied().unwrap_or(0.0),
            result.get(1).copied().unwrap_or(0.0),
            result.get(2).copied().unwrap_or(0.0),
        ]
    }

    /// Transform 4-channel input (CMYK)
    pub fn transform_cmyk(&self, cmyk: [f64; 4]) -> [f64; 4] {
        let result = self.transform(&cmyk);
        [
            result.first().copied().unwrap_or(0.0),
            result.get(1).copied().unwrap_or(0.0),
            result.get(2).copied().unwrap_or(0.0),
            result.get(3).copied().unwrap_or(0.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_curve_identity() {
        let curve = LutCurve::Identity;
        assert!((curve.eval(0.0) - 0.0).abs() < 1e-10);
        assert!((curve.eval(0.5) - 0.5).abs() < 1e-10);
        assert!((curve.eval(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_lut_curve_gamma() {
        let curve = LutCurve::Gamma(2.2);
        let result = curve.eval(0.5);
        let expected = 0.5_f64.powf(2.2);
        assert!((result - expected).abs() < 1e-10);
    }

    #[test]
    fn test_lut_curve_table() {
        // Linear table
        let table: Vec<f64> = (0..256).map(|i| i as f64 / 255.0).collect();
        let curve = LutCurve::Table(table);

        assert!((curve.eval(0.0) - 0.0).abs() < 0.01);
        assert!((curve.eval(0.5) - 0.5).abs() < 0.01);
        assert!((curve.eval(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_lut_pipeline_identity() {
        let pipeline = LutPipeline {
            input_channels: 3,
            output_channels: 3,
            input_curves: vec![LutCurve::Identity; 3],
            clut: None,
            output_curves: vec![LutCurve::Identity; 3],
            use_tetrahedral: true,
        };

        let input = [0.5, 0.3, 0.7];
        let output = pipeline.transform_rgb(input);

        assert!((output[0] - 0.5).abs() < 1e-10);
        assert!((output[1] - 0.3).abs() < 1e-10);
        assert!((output[2] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_parametric_curve_eval() {
        // Type 0: Y = X^g
        let result = eval_parametric(0, &[2.2], 0.5);
        assert!((result - 0.5_f64.powf(2.2)).abs() < 1e-10);

        // Type 3 (sRGB-like)
        let params = [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045];
        let result = eval_parametric(3, &params, 0.5);
        assert!(result > 0.0 && result < 1.0);
    }
}
