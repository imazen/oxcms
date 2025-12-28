//! Gamma and transfer function operations
//!
//! This module provides:
//! - sRGB gamma encode/decode
//! - ICC parametric curve types 0-4
//! - General gamma power functions

/// sRGB gamma decode (encoded → linear)
///
/// Converts sRGB-encoded value [0,1] to linear light [0,1].
/// Uses the IEC 61966-2-1 transfer function.
#[inline]
pub fn srgb_gamma_decode(encoded: f64) -> f64 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB gamma encode (linear → encoded)
///
/// Converts linear light [0,1] to sRGB-encoded value [0,1].
/// Uses the IEC 61966-2-1 transfer function.
#[inline]
pub fn srgb_gamma_encode(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Simple gamma power function (decode)
///
/// y = x^gamma
#[inline]
pub fn gamma_decode(encoded: f64, gamma: f64) -> f64 {
    if encoded <= 0.0 {
        0.0
    } else {
        encoded.powf(gamma)
    }
}

/// Simple gamma power function (encode)
///
/// y = x^(1/gamma)
#[inline]
pub fn gamma_encode(linear: f64, gamma: f64) -> f64 {
    if linear <= 0.0 {
        0.0
    } else {
        linear.powf(1.0 / gamma)
    }
}

/// ICC Parametric Curve Type
///
/// As defined in ICC.1:2022 Section 10.18
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParametricCurveType {
    /// Type 0: Y = X^g
    Gamma,
    /// Type 1: Y = (aX + b)^g  if X >= -b/a, else 0
    CIE122,
    /// Type 2: Y = (aX + b)^g + c  if X >= -b/a, else c
    IEC61966_3,
    /// Type 3: Y = (aX + b)^g  if X >= d, else cX (sRGB-like)
    IEC61966_2_1,
    /// Type 4: Y = (aX + b)^g + e  if X >= d, else cX + f
    Full,
}

impl ParametricCurveType {
    /// Get the function type from ICC value
    pub fn from_icc(function_type: u16) -> Option<Self> {
        match function_type {
            0 => Some(Self::Gamma),
            1 => Some(Self::CIE122),
            2 => Some(Self::IEC61966_3),
            3 => Some(Self::IEC61966_2_1),
            4 => Some(Self::Full),
            _ => None,
        }
    }

    /// Get the number of parameters required
    pub fn param_count(&self) -> usize {
        match self {
            Self::Gamma => 1,
            Self::CIE122 => 3,
            Self::IEC61966_3 => 4,
            Self::IEC61966_2_1 => 5,
            Self::Full => 7,
        }
    }
}

/// ICC Parametric Curve
///
/// Represents the 5 types of parametric curves defined in ICC.1:2022.
/// Parameters are stored as defined in the spec.
#[derive(Debug, Clone, Copy)]
pub struct ParametricCurve {
    /// Curve type (0-4)
    pub curve_type: ParametricCurveType,
    /// Gamma value (g)
    pub g: f64,
    /// Parameter a
    pub a: f64,
    /// Parameter b
    pub b: f64,
    /// Parameter c
    pub c: f64,
    /// Parameter d
    pub d: f64,
    /// Parameter e
    pub e: f64,
    /// Parameter f
    pub f: f64,
}

impl ParametricCurve {
    /// Create a simple gamma curve (type 0)
    pub fn gamma(g: f64) -> Self {
        Self {
            curve_type: ParametricCurveType::Gamma,
            g,
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create an sRGB transfer function curve (type 3)
    ///
    /// The sRGB transfer function is:
    /// - Y = (X/12.92)           if X <= 0.04045
    /// - Y = ((X+0.055)/1.055)^2.4  if X > 0.04045
    pub fn srgb() -> Self {
        Self {
            curve_type: ParametricCurveType::IEC61966_2_1,
            g: 2.4,
            a: 1.0 / 1.055,
            b: 0.055 / 1.055,
            c: 1.0 / 12.92,
            d: 0.04045,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create a BT.709/BT.2020 transfer function curve
    ///
    /// For decode: L = ((V + 0.099) / 1.099)^(1/0.45)  if V >= 0.081
    ///             L = V / 4.5                         if V < 0.081
    pub fn bt709() -> Self {
        Self {
            curve_type: ParametricCurveType::IEC61966_2_1,
            g: 1.0 / 0.45,
            a: 1.0 / 1.099,
            b: 0.099 / 1.099,
            c: 1.0 / 4.5,
            d: 0.081,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Create from ICC parameters
    pub fn from_params(curve_type: ParametricCurveType, params: &[f64]) -> Option<Self> {
        if params.len() < curve_type.param_count() {
            return None;
        }

        let mut curve = Self {
            curve_type,
            g: params.first().copied().unwrap_or(1.0),
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };

        match curve_type {
            ParametricCurveType::Gamma => {}
            ParametricCurveType::CIE122 => {
                curve.a = params[1];
                curve.b = params[2];
            }
            ParametricCurveType::IEC61966_3 => {
                curve.a = params[1];
                curve.b = params[2];
                curve.c = params[3];
            }
            ParametricCurveType::IEC61966_2_1 => {
                curve.a = params[1];
                curve.b = params[2];
                curve.c = params[3];
                curve.d = params[4];
            }
            ParametricCurveType::Full => {
                curve.a = params[1];
                curve.b = params[2];
                curve.c = params[3];
                curve.d = params[4];
                curve.e = params[5];
                curve.f = params[6];
            }
        }

        Some(curve)
    }
}

/// Evaluate a parametric curve (forward direction: encoded → linear)
///
/// This is the "decode" direction - converting encoded values to linear light.
#[inline]
pub fn parametric_curve_eval(curve: &ParametricCurve, x: f64) -> f64 {
    // Clamp input to [0, 1]
    let x = x.clamp(0.0, 1.0);

    match curve.curve_type {
        ParametricCurveType::Gamma => {
            // Y = X^g
            x.powf(curve.g)
        }
        ParametricCurveType::CIE122 => {
            // Y = (aX + b)^g  if X >= -b/a
            // Y = 0           if X < -b/a
            let threshold = if curve.a.abs() > 1e-10 {
                -curve.b / curve.a
            } else {
                0.0
            };
            if x >= threshold {
                (curve.a * x + curve.b).max(0.0).powf(curve.g)
            } else {
                0.0
            }
        }
        ParametricCurveType::IEC61966_3 => {
            // Y = (aX + b)^g + c  if X >= -b/a
            // Y = c               if X < -b/a
            let threshold = if curve.a.abs() > 1e-10 {
                -curve.b / curve.a
            } else {
                0.0
            };
            if x >= threshold {
                (curve.a * x + curve.b).max(0.0).powf(curve.g) + curve.c
            } else {
                curve.c
            }
        }
        ParametricCurveType::IEC61966_2_1 => {
            // Y = (aX + b)^g  if X >= d
            // Y = cX          if X < d
            if x >= curve.d {
                (curve.a * x + curve.b).max(0.0).powf(curve.g)
            } else {
                curve.c * x
            }
        }
        ParametricCurveType::Full => {
            // Y = (aX + b)^g + e  if X >= d
            // Y = cX + f          if X < d
            if x >= curve.d {
                (curve.a * x + curve.b).max(0.0).powf(curve.g) + curve.e
            } else {
                curve.c * x + curve.f
            }
        }
    }
}

/// Evaluate a parametric curve in reverse (linear → encoded)
///
/// This is the "encode" direction - converting linear light to encoded values.
/// Note: Not all curve types have closed-form inverses; this uses approximations.
#[inline]
pub fn parametric_curve_eval_inverse(curve: &ParametricCurve, y: f64) -> f64 {
    // Clamp input to valid range
    let y = y.clamp(0.0, 1.0);

    match curve.curve_type {
        ParametricCurveType::Gamma => {
            // X = Y^(1/g)
            if curve.g.abs() > 1e-10 {
                y.powf(1.0 / curve.g)
            } else {
                y
            }
        }
        ParametricCurveType::IEC61966_2_1 => {
            // Inverse of: Y = (aX + b)^g if X >= d, else cX
            // Linear segment inverse: X = Y/c
            // Power segment inverse: X = (Y^(1/g) - b) / a
            let linear_threshold = curve.c * curve.d;
            if y < linear_threshold {
                if curve.c.abs() > 1e-10 {
                    y / curve.c
                } else {
                    0.0
                }
            } else if curve.a.abs() > 1e-10 && curve.g.abs() > 1e-10 {
                (y.powf(1.0 / curve.g) - curve.b) / curve.a
            } else {
                y
            }
        }
        _ => {
            // For other types, use Newton-Raphson iteration
            // Starting guess: y^(1/g) is usually close
            let mut x = if curve.g.abs() > 1e-10 {
                y.powf(1.0 / curve.g)
            } else {
                y
            };

            // Newton-Raphson iterations
            for _ in 0..8 {
                let fx = parametric_curve_eval(curve, x) - y;
                if fx.abs() < 1e-12 {
                    break;
                }
                // Numerical derivative
                let h = 1e-8;
                let dfx = (parametric_curve_eval(curve, x + h) - parametric_curve_eval(curve, x - h))
                    / (2.0 * h);
                if dfx.abs() > 1e-10 {
                    x -= fx / dfx;
                    x = x.clamp(0.0, 1.0);
                }
            }
            x
        }
    }
}

/// Build a lookup table for a parametric curve
///
/// Returns a Vec of `size` entries mapping [0, size-1] to [0.0, 1.0] output.
pub fn build_curve_lut(curve: &ParametricCurve, size: usize) -> Vec<f64> {
    let mut lut = Vec::with_capacity(size);
    for i in 0..size {
        let x = i as f64 / (size - 1) as f64;
        lut.push(parametric_curve_eval(curve, x));
    }
    lut
}

/// Build an inverse lookup table for a parametric curve
pub fn build_curve_lut_inverse(curve: &ParametricCurve, size: usize) -> Vec<f64> {
    let mut lut = Vec::with_capacity(size);
    for i in 0..size {
        let y = i as f64 / (size - 1) as f64;
        lut.push(parametric_curve_eval_inverse(curve, y));
    }
    lut
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_srgb_gamma_roundtrip() {
        for i in 0..=255 {
            let encoded = i as f64 / 255.0;
            let linear = srgb_gamma_decode(encoded);
            let roundtrip = srgb_gamma_encode(linear);
            assert!(
                (roundtrip - encoded).abs() < 1e-10,
                "sRGB roundtrip failed at {}",
                i
            );
        }
    }

    #[test]
    fn test_srgb_known_values() {
        // Black stays black
        assert!((srgb_gamma_decode(0.0) - 0.0).abs() < EPSILON);
        // White stays white
        assert!((srgb_gamma_decode(1.0) - 1.0).abs() < EPSILON);

        // Mid-gray: 0.5 encoded → ~0.214 linear (sRGB is darker than gamma 2.2)
        let mid = srgb_gamma_decode(0.5);
        assert!(mid > 0.21 && mid < 0.22, "Mid-gray decode: {}", mid);

        // Verify the linear segment
        assert!((srgb_gamma_decode(0.04045) - 0.04045 / 12.92).abs() < 1e-10);
    }

    #[test]
    fn test_simple_gamma() {
        let gamma = 2.2;

        // Roundtrip
        for i in 0..=255 {
            let encoded = i as f64 / 255.0;
            let linear = gamma_decode(encoded, gamma);
            let roundtrip = gamma_encode(linear, gamma);
            assert!(
                (roundtrip - encoded).abs() < 1e-10,
                "gamma roundtrip failed at {}",
                i
            );
        }
    }

    #[test]
    fn test_parametric_type0() {
        let curve = ParametricCurve::gamma(2.2);
        let x = 0.5;
        let y = parametric_curve_eval(&curve, x);
        let expected = 0.5_f64.powf(2.2);
        assert!((y - expected).abs() < EPSILON);
    }

    #[test]
    fn test_parametric_srgb() {
        let curve = ParametricCurve::srgb();

        // Compare to reference srgb_gamma_decode
        for i in 0..=255 {
            let x = i as f64 / 255.0;
            let parametric = parametric_curve_eval(&curve, x);
            let reference = srgb_gamma_decode(x);
            assert!(
                (parametric - reference).abs() < 1e-9,
                "sRGB parametric mismatch at {}: {} vs {}",
                i,
                parametric,
                reference
            );
        }
    }

    #[test]
    fn test_parametric_srgb_inverse() {
        let curve = ParametricCurve::srgb();

        // Roundtrip test
        for i in 0..=255 {
            let x = i as f64 / 255.0;
            let y = parametric_curve_eval(&curve, x);
            let roundtrip = parametric_curve_eval_inverse(&curve, y);
            assert!(
                (roundtrip - x).abs() < 1e-8,
                "sRGB inverse failed at {}: {} -> {} -> {}",
                i,
                x,
                y,
                roundtrip
            );
        }
    }

    #[test]
    fn test_curve_lut() {
        let curve = ParametricCurve::srgb();
        let lut = build_curve_lut(&curve, 256);

        assert_eq!(lut.len(), 256);
        assert!((lut[0] - 0.0).abs() < EPSILON); // Black
        assert!((lut[255] - 1.0).abs() < EPSILON); // White

        // Compare to direct evaluation
        for (i, &val) in lut.iter().enumerate() {
            let x = i as f64 / 255.0;
            let direct = parametric_curve_eval(&curve, x);
            assert!((val - direct).abs() < EPSILON);
        }
    }

    #[test]
    fn test_param_count() {
        assert_eq!(ParametricCurveType::Gamma.param_count(), 1);
        assert_eq!(ParametricCurveType::CIE122.param_count(), 3);
        assert_eq!(ParametricCurveType::IEC61966_3.param_count(), 4);
        assert_eq!(ParametricCurveType::IEC61966_2_1.param_count(), 5);
        assert_eq!(ParametricCurveType::Full.param_count(), 7);
    }
}
