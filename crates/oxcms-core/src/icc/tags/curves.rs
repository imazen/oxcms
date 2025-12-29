//! Curve Tag Types
//!
//! ICC profiles use curves for tone reproduction (TRC).
//! Two main types:
//! - curv: Simple gamma or lookup table
//! - para: Parametric curves with formula
//!
//! See ICC.1:2022 Sections 10.6 (curv) and 10.18 (para)

use crate::icc::error::IccError;
use crate::math::gamma::{ParametricCurve, ParametricCurveType};

/// Curve tag data (curv type)
#[derive(Debug, Clone)]
pub enum CurveData {
    /// Identity curve (count = 0)
    Identity,
    /// Simple gamma (count = 1, value is u8Fixed8)
    Gamma(f64),
    /// Lookup table (count > 1, values are u16)
    Table(Vec<u16>),
}

impl CurveData {
    /// Parse curve data from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 4 {
            return Err(IccError::CorruptedData("Curve tag too small".to_string()));
        }

        let count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

        match count {
            0 => Ok(CurveData::Identity),
            1 => {
                // Single gamma value as u8Fixed8
                if data.len() < 6 {
                    return Err(IccError::CorruptedData(
                        "Curve gamma value missing".to_string(),
                    ));
                }
                let gamma_raw = u16::from_be_bytes([data[4], data[5]]);
                let gamma = gamma_raw as f64 / 256.0;
                Ok(CurveData::Gamma(gamma))
            }
            _ => {
                // Table of u16 values
                let required_len = 4 + count * 2;
                if data.len() < required_len {
                    return Err(IccError::CorruptedData(format!(
                        "Curve table too small: need {} bytes, have {}",
                        required_len,
                        data.len()
                    )));
                }

                let mut table = Vec::with_capacity(count);
                for i in 0..count {
                    let offset = 4 + i * 2;
                    let val = u16::from_be_bytes([data[offset], data[offset + 1]]);
                    table.push(val);
                }
                Ok(CurveData::Table(table))
            }
        }
    }

    /// Evaluate the curve at a given input (0.0 to 1.0)
    pub fn eval(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);

        match self {
            CurveData::Identity => x,
            CurveData::Gamma(g) => x.powf(*g),
            CurveData::Table(table) => {
                if table.is_empty() {
                    return x;
                }
                if table.len() == 1 {
                    return table[0] as f64 / 65535.0;
                }

                // Linear interpolation in the table
                let pos = x * (table.len() - 1) as f64;
                let idx = pos.floor() as usize;
                let frac = pos - idx as f64;

                if idx >= table.len() - 1 {
                    return table[table.len() - 1] as f64 / 65535.0;
                }

                let v0 = table[idx] as f64;
                let v1 = table[idx + 1] as f64;
                (v0 + frac * (v1 - v0)) / 65535.0
            }
        }
    }

    /// Evaluate the inverse curve (for encoding)
    pub fn eval_inverse(&self, y: f64) -> f64 {
        let y = y.clamp(0.0, 1.0);

        match self {
            CurveData::Identity => y,
            CurveData::Gamma(g) => {
                if *g == 0.0 {
                    return y;
                }
                y.powf(1.0 / *g)
            }
            CurveData::Table(table) => {
                // Binary search for inverse
                if table.is_empty() {
                    return y;
                }
                let target = (y * 65535.0) as u16;

                // Find the position where target would be
                let mut lo = 0usize;
                let mut hi = table.len() - 1;

                while lo < hi {
                    let mid = (lo + hi) / 2;
                    if table[mid] < target {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }

                // Interpolate
                if lo == 0 {
                    return 0.0;
                }
                if lo >= table.len() {
                    return 1.0;
                }

                let v0 = table[lo - 1] as f64;
                let v1 = table[lo] as f64;
                let t = (y * 65535.0 - v0) / (v1 - v0);
                ((lo - 1) as f64 + t) / (table.len() - 1) as f64
            }
        }
    }

    /// Check if this is a linear (identity) curve
    pub fn is_linear(&self) -> bool {
        match self {
            CurveData::Identity => true,
            CurveData::Gamma(g) => (*g - 1.0).abs() < 1e-6,
            CurveData::Table(table) => {
                // Check if table is linear ramp
                for (i, &v) in table.iter().enumerate() {
                    let expected = (i as f64 / (table.len() - 1) as f64 * 65535.0) as u16;
                    if (v as i32 - expected as i32).abs() > 1 {
                        return false;
                    }
                }
                true
            }
        }
    }
}

/// Parametric curve data (para type)
#[derive(Debug, Clone)]
pub struct ParametricCurveData {
    /// The parametric curve
    pub curve: ParametricCurve,
}

impl ParametricCurveData {
    /// Parse parametric curve from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 4 {
            return Err(IccError::CorruptedData(
                "Parametric curve too small".to_string(),
            ));
        }

        let func_type = u16::from_be_bytes([data[0], data[1]]);
        // Bytes 2-3 are reserved

        let curve_type = match func_type {
            0 => ParametricCurveType::Gamma,
            1 => ParametricCurveType::CIE122,
            2 => ParametricCurveType::IEC61966_3,
            3 => ParametricCurveType::IEC61966_2_1,
            4 => ParametricCurveType::Full,
            _ => {
                return Err(IccError::CorruptedData(format!(
                    "Unknown parametric curve type: {}",
                    func_type
                )));
            }
        };

        // Parse parameters (s15Fixed16)
        let param_offset = 4;
        let parse_s15f16 = |offset: usize| -> f64 {
            if data.len() < offset + 4 {
                return 0.0;
            }
            let raw = i32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            raw as f64 / 65536.0
        };

        let g = parse_s15f16(param_offset);
        let a = parse_s15f16(param_offset + 4);
        let b = parse_s15f16(param_offset + 8);
        let c = parse_s15f16(param_offset + 12);
        let d = parse_s15f16(param_offset + 16);
        let e = parse_s15f16(param_offset + 20);
        let f = parse_s15f16(param_offset + 24);

        Ok(Self {
            curve: ParametricCurve {
                curve_type,
                g,
                a,
                b,
                c,
                d,
                e,
                f,
            },
        })
    }

    /// Evaluate the curve at a given input
    pub fn eval(&self, x: f64) -> f64 {
        crate::math::gamma::parametric_curve_eval(&self.curve, x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_identity() {
        let data: [u8; 4] = [0, 0, 0, 0]; // count = 0
        let curve = CurveData::parse(&data).unwrap();
        assert!(matches!(curve, CurveData::Identity));
        assert!(curve.is_linear());
        assert!((curve.eval(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_curve_gamma() {
        // Gamma 2.2 as u8Fixed8 = 2.2 * 256 = 563.2 ≈ 563 = 0x0233
        let data: [u8; 6] = [
            0, 0, 0, 1, // count = 1
            0x02, 0x33, // gamma = 563/256 ≈ 2.199
        ];
        let curve = CurveData::parse(&data).unwrap();

        if let CurveData::Gamma(g) = curve {
            assert!((g - 2.199).abs() < 0.01);
        } else {
            panic!("Expected Gamma curve");
        }
    }

    #[test]
    fn test_curve_table() {
        // Small 3-entry table
        let data: [u8; 10] = [
            0, 0, 0, 3, // count = 3
            0x00, 0x00, // 0
            0x80, 0x00, // 32768
            0xFF, 0xFF, // 65535
        ];
        let curve = CurveData::parse(&data).unwrap();

        if let CurveData::Table(table) = &curve {
            assert_eq!(table.len(), 3);
            assert_eq!(table[0], 0);
            assert_eq!(table[1], 0x8000);
            assert_eq!(table[2], 0xFFFF);
        } else {
            panic!("Expected Table curve");
        }

        // Test interpolation
        assert!((curve.eval(0.0) - 0.0).abs() < 0.001);
        assert!((curve.eval(0.5) - 0.5).abs() < 0.001);
        assert!((curve.eval(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_parametric_type0() {
        // Type 0: Y = X^g, with g = 2.2
        let mut data = vec![
            0, 0, // function type 0
            0, 0, // reserved
        ];
        // g = 2.2 as s15Fixed16 = 2.2 * 65536 = 144179.2 ≈ 0x00023333
        data.extend_from_slice(&[0x00, 0x02, 0x33, 0x33]);

        let curve = ParametricCurveData::parse(&data).unwrap();
        assert_eq!(curve.curve.curve_type, ParametricCurveType::Gamma);
        assert!((curve.curve.g - 2.2).abs() < 0.001);

        // Test evaluation: 0.5^2.2 ≈ 0.2176
        let result = curve.eval(0.5);
        assert!((result - 0.2176).abs() < 0.001);
    }
}
