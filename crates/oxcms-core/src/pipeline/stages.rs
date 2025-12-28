//! Pipeline Stages
//!
//! Individual components of a color transform pipeline.

use crate::icc::CurveData;
use crate::math::Matrix3x3;

/// A pipeline stage
#[derive(Debug, Clone)]
pub enum PipelineStage {
    /// TRC (Tone Reproduction Curve) stage
    Trc(TrcStage),
    /// Matrix stage
    Matrix(MatrixStage),
    /// Chromatic adaptation stage
    ChromaticAdaptation(Matrix3x3),
    /// Clamp stage
    Clamp,
}

impl PipelineStage {
    /// Apply the stage to RGB values
    pub fn apply_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        match self {
            PipelineStage::Trc(trc) => trc.apply(rgb),
            PipelineStage::Matrix(mat) => mat.apply(rgb),
            PipelineStage::ChromaticAdaptation(mat) => mat.multiply_vec(rgb),
            PipelineStage::Clamp => [
                rgb[0].clamp(0.0, 1.0),
                rgb[1].clamp(0.0, 1.0),
                rgb[2].clamp(0.0, 1.0),
            ],
        }
    }

    /// Apply inverse of the stage
    pub fn apply_inverse_rgb(&self, rgb: [f64; 3]) -> [f64; 3] {
        match self {
            PipelineStage::Trc(trc) => trc.apply_inverse(rgb),
            PipelineStage::Matrix(mat) => mat.apply_inverse(rgb),
            PipelineStage::ChromaticAdaptation(mat) => {
                mat.inverse().map_or(rgb, |inv| inv.multiply_vec(rgb))
            }
            PipelineStage::Clamp => rgb, // No inverse for clamp
        }
    }
}

/// TRC (Tone Reproduction Curve) stage
///
/// Applies transfer functions to each channel.
#[derive(Debug, Clone)]
pub struct TrcStage {
    /// Red channel curve
    pub red: TrcCurve,
    /// Green channel curve
    pub green: TrcCurve,
    /// Blue channel curve
    pub blue: TrcCurve,
}

impl TrcStage {
    /// Create from ICC curves
    pub fn from_curves(
        red: Option<&CurveData>,
        green: Option<&CurveData>,
        blue: Option<&CurveData>,
    ) -> Self {
        Self {
            red: red.map(TrcCurve::from_icc).unwrap_or_default(),
            green: green.map(TrcCurve::from_icc).unwrap_or_default(),
            blue: blue.map(TrcCurve::from_icc).unwrap_or_default(),
        }
    }

    /// Apply TRCs to decode (encoded → linear)
    pub fn apply(&self, rgb: [f64; 3]) -> [f64; 3] {
        [
            self.red.decode(rgb[0]),
            self.green.decode(rgb[1]),
            self.blue.decode(rgb[2]),
        ]
    }

    /// Apply inverse TRCs to encode (linear → encoded)
    pub fn apply_inverse(&self, rgb: [f64; 3]) -> [f64; 3] {
        [
            self.red.encode(rgb[0]),
            self.green.encode(rgb[1]),
            self.blue.encode(rgb[2]),
        ]
    }
}

/// A single TRC curve
#[derive(Debug, Clone)]
pub enum TrcCurve {
    /// Identity (linear)
    Identity,
    /// Simple gamma
    Gamma(f64),
    /// Lookup table (normalized to f64)
    Table(Vec<f64>),
}

impl Default for TrcCurve {
    fn default() -> Self {
        TrcCurve::Identity
    }
}

impl TrcCurve {
    /// Create from ICC curve data
    pub fn from_icc(curve: &CurveData) -> Self {
        match curve {
            CurveData::Identity => TrcCurve::Identity,
            CurveData::Gamma(g) => TrcCurve::Gamma(*g),
            CurveData::Table(table) => {
                // Normalize to f64
                let normalized: Vec<f64> = table.iter().map(|&v| v as f64 / 65535.0).collect();
                TrcCurve::Table(normalized)
            }
        }
    }

    /// Decode (apply forward curve: encoded → linear)
    pub fn decode(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        match self {
            TrcCurve::Identity => x,
            TrcCurve::Gamma(g) => x.powf(*g),
            TrcCurve::Table(table) => {
                if table.is_empty() {
                    return x;
                }
                if table.len() == 1 {
                    return table[0];
                }

                // Linear interpolation
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

    /// Encode (apply inverse curve: linear → encoded)
    pub fn encode(&self, y: f64) -> f64 {
        let y = y.clamp(0.0, 1.0);
        match self {
            TrcCurve::Identity => y,
            TrcCurve::Gamma(g) => {
                if *g == 0.0 {
                    y
                } else {
                    y.powf(1.0 / *g)
                }
            }
            TrcCurve::Table(table) => {
                // Binary search for inverse
                if table.is_empty() || table.len() == 1 {
                    return y;
                }

                // Find position where value would be
                let mut lo = 0usize;
                let mut hi = table.len() - 1;

                while lo < hi {
                    let mid = (lo + hi) / 2;
                    if table[mid] < y {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }

                if lo == 0 {
                    return 0.0;
                }

                // Interpolate
                let v0 = table[lo - 1];
                let v1 = table[lo];
                let t = if (v1 - v0).abs() > 1e-10 {
                    (y - v0) / (v1 - v0)
                } else {
                    0.0
                };

                ((lo - 1) as f64 + t) / (table.len() - 1) as f64
            }
        }
    }
}

/// Matrix stage
///
/// Applies a 3x3 matrix transformation.
#[derive(Debug, Clone)]
pub struct MatrixStage {
    /// Forward matrix
    pub matrix: Matrix3x3,
    /// Inverse matrix (cached)
    pub inverse: Option<Matrix3x3>,
}

impl MatrixStage {
    /// Create from matrix
    pub fn new(matrix: Matrix3x3) -> Self {
        let inverse = matrix.inverse();
        Self { matrix, inverse }
    }

    /// Apply forward matrix
    pub fn apply(&self, rgb: [f64; 3]) -> [f64; 3] {
        self.matrix.multiply_vec(rgb)
    }

    /// Apply inverse matrix
    pub fn apply_inverse(&self, rgb: [f64; 3]) -> [f64; 3] {
        self.inverse
            .as_ref()
            .map_or(rgb, |inv| inv.multiply_vec(rgb))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trc_identity() {
        let trc = TrcCurve::Identity;
        assert!((trc.decode(0.5) - 0.5).abs() < 1e-10);
        assert!((trc.encode(0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_trc_gamma() {
        let trc = TrcCurve::Gamma(2.2);

        // Decode: x^2.2
        let decoded = trc.decode(0.5);
        assert!((decoded - 0.5_f64.powf(2.2)).abs() < 1e-10);

        // Encode: x^(1/2.2)
        let encoded = trc.encode(0.5);
        assert!((encoded - 0.5_f64.powf(1.0 / 2.2)).abs() < 1e-10);

        // Roundtrip
        for i in 0..=255 {
            let x = i as f64 / 255.0;
            let decoded = trc.decode(x);
            let roundtrip = trc.encode(decoded);
            assert!(
                (roundtrip - x).abs() < 1e-9,
                "Roundtrip failed at {}: {} -> {} -> {}",
                i,
                x,
                decoded,
                roundtrip
            );
        }
    }

    #[test]
    fn test_trc_table() {
        // Simple linear table
        let table: Vec<f64> = (0..256).map(|i| i as f64 / 255.0).collect();
        let trc = TrcCurve::Table(table);

        // Should be identity
        assert!((trc.decode(0.5) - 0.5).abs() < 0.01);
        assert!((trc.encode(0.5) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_matrix_stage() {
        let matrix = Matrix3x3::identity();
        let stage = MatrixStage::new(matrix);

        let rgb = [0.5, 0.3, 0.7];
        let result = stage.apply(rgb);

        assert!((result[0] - rgb[0]).abs() < 1e-10);
        assert!((result[1] - rgb[1]).abs() < 1e-10);
        assert!((result[2] - rgb[2]).abs() < 1e-10);
    }

    #[test]
    fn test_trc_stage() {
        let stage = TrcStage {
            red: TrcCurve::Gamma(2.2),
            green: TrcCurve::Gamma(2.2),
            blue: TrcCurve::Gamma(2.2),
        };

        let rgb = [0.5, 0.5, 0.5];
        let decoded = stage.apply(rgb);
        let roundtrip = stage.apply_inverse(decoded);

        assert!((roundtrip[0] - rgb[0]).abs() < 1e-9);
        assert!((roundtrip[1] - rgb[1]).abs() < 1e-9);
        assert!((roundtrip[2] - rgb[2]).abs() < 1e-9);
    }
}
