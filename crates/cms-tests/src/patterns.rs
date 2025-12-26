//! Test pattern generation
//!
//! Provides various test patterns for comprehensive evaluation.

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// Test pattern types
#[derive(Debug, Clone, Copy)]
pub enum TestPattern {
    /// Horizontal gradient black to white
    GradientH,
    /// Vertical gradient black to white
    GradientV,
    /// RGB color cube corners (8 colors)
    ColorCube,
    /// Hue ramp at full saturation
    HueRamp,
    /// Grayscale ramp 0-255
    Grayscale,
    /// Random pixels with seed
    Random(u64),
    /// Skin tone samples
    SkinTones,
    /// Saturated colors near gamut boundary
    GamutBoundary,
    /// All zeros (black)
    Black,
    /// All 255 (white)
    White,
}

/// Generate test pattern as RGB8 buffer
pub fn generate_pattern(pattern: TestPattern, width: usize, height: usize) -> Vec<u8> {
    let pixel_count = width * height;
    let mut data = vec![0u8; pixel_count * 3];

    match pattern {
        TestPattern::GradientH => {
            for y in 0..height {
                for x in 0..width {
                    let v = ((x as f32 / width as f32) * 255.0) as u8;
                    let idx = (y * width + x) * 3;
                    data[idx] = v;
                    data[idx + 1] = v;
                    data[idx + 2] = v;
                }
            }
        }
        TestPattern::GradientV => {
            for y in 0..height {
                let v = ((y as f32 / height as f32) * 255.0) as u8;
                for x in 0..width {
                    let idx = (y * width + x) * 3;
                    data[idx] = v;
                    data[idx + 1] = v;
                    data[idx + 2] = v;
                }
            }
        }
        TestPattern::ColorCube => {
            let corners: [[u8; 3]; 8] = [
                [0, 0, 0],
                [255, 0, 0],
                [0, 255, 0],
                [0, 0, 255],
                [255, 255, 0],
                [255, 0, 255],
                [0, 255, 255],
                [255, 255, 255],
            ];
            for (i, chunk) in data.chunks_exact_mut(3).enumerate() {
                let c = corners[i % 8];
                chunk.copy_from_slice(&c);
            }
        }
        TestPattern::HueRamp => {
            for (i, chunk) in data.chunks_exact_mut(3).enumerate() {
                let hue = (i as f32 / pixel_count as f32) * 360.0;
                let (r, g, b) = hsl_to_rgb(hue, 1.0, 0.5);
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
            }
        }
        TestPattern::Grayscale => {
            for (i, chunk) in data.chunks_exact_mut(3).enumerate() {
                let v = ((i as f32 / pixel_count as f32) * 255.0) as u8;
                chunk[0] = v;
                chunk[1] = v;
                chunk[2] = v;
            }
        }
        TestPattern::Random(seed) => {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            rng.fill_bytes(&mut data);
        }
        TestPattern::SkinTones => {
            let tones: [[u8; 3]; 6] = [
                [255, 224, 189],
                [241, 194, 125],
                [224, 172, 105],
                [198, 134, 66],
                [141, 85, 36],
                [89, 47, 42],
            ];
            for (i, chunk) in data.chunks_exact_mut(3).enumerate() {
                chunk.copy_from_slice(&tones[i % 6]);
            }
        }
        TestPattern::GamutBoundary => {
            let colors: [[u8; 3]; 8] = [
                [255, 0, 0],
                [0, 255, 0],
                [0, 0, 255],
                [255, 255, 0],
                [255, 0, 255],
                [0, 255, 255],
                [255, 128, 0],
                [128, 0, 255],
            ];
            for (i, chunk) in data.chunks_exact_mut(3).enumerate() {
                chunk.copy_from_slice(&colors[i % 8]);
            }
        }
        TestPattern::Black => {
            // Already zeros
        }
        TestPattern::White => {
            data.fill(255);
        }
    }

    data
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

/// Standard test sizes
pub mod sizes {
    pub const TINY: (usize, usize) = (8, 8);
    pub const SMALL: (usize, usize) = (64, 64);
    pub const MEDIUM: (usize, usize) = (256, 256);
    pub const LARGE: (usize, usize) = (1920, 1080);
    pub const HUGE: (usize, usize) = (4096, 4096);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_black() {
        let data = generate_pattern(TestPattern::Black, 2, 2);
        assert!(data.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_generate_white() {
        let data = generate_pattern(TestPattern::White, 2, 2);
        assert!(data.iter().all(|&v| v == 255));
    }

    #[test]
    fn test_random_deterministic() {
        let a = generate_pattern(TestPattern::Random(42), 10, 10);
        let b = generate_pattern(TestPattern::Random(42), 10, 10);
        assert_eq!(a, b);
    }
}
