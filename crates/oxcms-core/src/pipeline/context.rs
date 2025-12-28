//! Transform Context and Options
//!
//! Configuration options for color transforms.

use crate::color::WhitePoint;
use crate::math::ChromaticAdaptationMethod;

/// Rendering intent for color conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderIntent {
    /// Perceptual - best for photos, maintains relative appearance
    #[default]
    Perceptual,
    /// Relative colorimetric - preserves in-gamut colors exactly
    RelativeColorimetric,
    /// Saturation - maintains saturation, good for business graphics
    Saturation,
    /// Absolute colorimetric - preserves white point
    AbsoluteColorimetric,
}

impl RenderIntent {
    /// Convert from ICC rendering intent value
    pub fn from_icc(value: u32) -> Self {
        match value {
            0 => Self::Perceptual,
            1 => Self::RelativeColorimetric,
            2 => Self::Saturation,
            3 => Self::AbsoluteColorimetric,
            _ => Self::Perceptual,
        }
    }

    /// Convert to ICC rendering intent value
    pub fn to_icc(&self) -> u32 {
        match self {
            Self::Perceptual => 0,
            Self::RelativeColorimetric => 1,
            Self::Saturation => 2,
            Self::AbsoluteColorimetric => 3,
        }
    }
}

/// Transform flags for additional options
#[derive(Debug, Clone, Copy, Default)]
pub struct TransformFlags {
    /// Use black point compensation
    pub black_point_compensation: bool,
    /// Clamp output to valid range
    pub clamp_output: bool,
    /// Enable soft proofing (preview)
    pub soft_proof: bool,
    /// Gamut check mode
    pub gamut_check: bool,
    /// Use high precision (f64 vs f32)
    pub high_precision: bool,
}

impl TransformFlags {
    /// Create default flags (clamping enabled)
    pub fn new() -> Self {
        Self {
            clamp_output: true,
            ..Default::default()
        }
    }

    /// Enable black point compensation
    pub fn with_bpc(mut self) -> Self {
        self.black_point_compensation = true;
        self
    }

    /// Enable soft proofing
    pub fn with_soft_proof(mut self) -> Self {
        self.soft_proof = true;
        self
    }
}

/// Transform context containing all configuration
#[derive(Debug, Clone)]
pub struct TransformContext {
    /// Rendering intent
    pub intent: RenderIntent,
    /// Transform flags
    pub flags: TransformFlags,
    /// Chromatic adaptation method
    pub adaptation_method: ChromaticAdaptationMethod,
    /// PCS white point (usually D50)
    pub pcs_white: WhitePoint,
    /// Gamut warning color (for gamut check mode)
    pub gamut_warning_color: [f64; 3],
}

impl Default for TransformContext {
    fn default() -> Self {
        use crate::color::white_point::D50;

        Self {
            intent: RenderIntent::default(),
            flags: TransformFlags::new(),
            adaptation_method: ChromaticAdaptationMethod::Bradford,
            pcs_white: D50,
            gamut_warning_color: [1.0, 0.0, 1.0], // Magenta
        }
    }
}

impl TransformContext {
    /// Create a new context with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set rendering intent
    pub fn with_intent(mut self, intent: RenderIntent) -> Self {
        self.intent = intent;
        self
    }

    /// Set flags
    pub fn with_flags(mut self, flags: TransformFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Enable black point compensation
    pub fn with_bpc(mut self) -> Self {
        self.flags.black_point_compensation = true;
        self
    }

    /// Set chromatic adaptation method
    pub fn with_adaptation(mut self, method: ChromaticAdaptationMethod) -> Self {
        self.adaptation_method = method;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_intent_roundtrip() {
        for i in 0..4 {
            let intent = RenderIntent::from_icc(i);
            assert_eq!(intent.to_icc(), i);
        }
    }

    #[test]
    fn test_context_builder() {
        let ctx = TransformContext::new()
            .with_intent(RenderIntent::RelativeColorimetric)
            .with_bpc();

        assert_eq!(ctx.intent, RenderIntent::RelativeColorimetric);
        assert!(ctx.flags.black_point_compensation);
    }
}
