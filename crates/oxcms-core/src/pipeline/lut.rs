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

use crate::icc::tags::{Lut16Data, Lut8Data, LutAToBData, LutBToAData, CurveSegment};
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
        // Currently only support 3-channel input for simplicity
        if input.len() != 3 || clut.grid_points.len() < 3 {
            // For other dimensions, fall back to simpler interpolation
            return self.apply_clut_generic(input, clut);
        }

        let grid_size = clut.grid_points[0];

        // Use tetrahedral or trilinear interpolation
        if self.use_tetrahedral {
            let result = tetrahedral_interp(&clut.data, grid_size, [input[0], input[1], input[2]]);
            result.to_vec()
        } else {
            let result = trilinear_interp(&clut.data, grid_size, [input[0], input[1], input[2]]);
            result.to_vec()
        }
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
