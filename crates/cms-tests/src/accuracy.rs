//! Accuracy measurement using perceptual color difference metrics
//!
//! Uses CIEDE2000 (deltaE2000) as the primary metric for color difference.
//! This is superior to MSE/PSNR which don't correlate with human perception.

/// Statistics from a deltaE comparison
#[derive(Debug, Clone)]
pub struct DeltaEStats {
    /// Mean deltaE across all samples
    pub mean: f64,
    /// Maximum deltaE
    pub max: f64,
    /// 95th percentile deltaE
    pub p95: f64,
    /// Number of samples
    pub count: usize,
}

impl DeltaEStats {
    /// Check if all differences are imperceptible (deltaE < 1.0)
    pub fn is_excellent(&self) -> bool {
        self.max < 1.0
    }

    /// Check if differences are barely perceptible (deltaE < 2.0)
    pub fn is_good(&self) -> bool {
        self.max < 2.0
    }

    /// Check if differences are acceptable (deltaE < 3.5)
    pub fn is_acceptable(&self) -> bool {
        self.max < 3.5
    }
}

/// Calculate deltaE2000 between two Lab colors
///
/// This is the industry-standard color difference formula that correlates
/// well with human perception. A deltaE2000 of 1.0 is roughly the smallest
/// difference perceptible to trained observers.
pub fn delta_e_2000(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let l1 = lab1[0];
    let a1 = lab1[1];
    let b1 = lab1[2];
    let l2 = lab2[0];
    let a2 = lab2[1];
    let b2 = lab2[2];

    // Parametric weighting factors
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    // Calculate C* (chroma)
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_avg = (c1 + c2) / 2.0;

    // Calculate G (adjustment factor for a*)
    let c_avg_pow7 = c_avg.powi(7);
    let g = 0.5 * (1.0 - (c_avg_pow7 / (c_avg_pow7 + 6103515625.0_f64)).sqrt()); // 25^7

    // Adjusted a* values
    let a1_prime = a1 * (1.0 + g);
    let a2_prime = a2 * (1.0 + g);

    // Calculate C'
    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();
    let c_avg_prime = (c1_prime + c2_prime) / 2.0;

    // Calculate h' (hue angle)
    let h1_prime = if a1_prime == 0.0 && b1 == 0.0 {
        0.0
    } else {
        let mut h = b1.atan2(a1_prime).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        h
    };

    let h2_prime = if a2_prime == 0.0 && b2 == 0.0 {
        0.0
    } else {
        let mut h = b2.atan2(a2_prime).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        h
    };

    // Calculate delta h'
    let delta_h_prime = if c1_prime * c2_prime == 0.0 {
        0.0
    } else {
        let diff = h2_prime - h1_prime;
        if diff.abs() <= 180.0 {
            diff
        } else if diff > 180.0 {
            diff - 360.0
        } else {
            diff + 360.0
        }
    };

    // Calculate Delta H'
    let delta_h_prime_big =
        2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime.to_radians() / 2.0).sin();

    // Calculate H' average
    let h_avg_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else {
        let diff = (h1_prime - h2_prime).abs();
        if diff <= 180.0 {
            (h1_prime + h2_prime) / 2.0
        } else if h1_prime + h2_prime < 360.0 {
            (h1_prime + h2_prime + 360.0) / 2.0
        } else {
            (h1_prime + h2_prime - 360.0) / 2.0
        }
    };

    // Calculate T
    let t = 1.0 - 0.17 * (h_avg_prime - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_avg_prime).to_radians().cos()
        + 0.32 * (3.0 * h_avg_prime + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_avg_prime - 63.0).to_radians().cos();

    // Calculate delta L', delta C'
    let delta_l_prime = l2 - l1;
    let delta_c_prime = c2_prime - c1_prime;

    // Calculate L' average
    let l_avg_prime = (l1 + l2) / 2.0;

    // Calculate S_L, S_C, S_H
    let l_avg_minus_50_sq = (l_avg_prime - 50.0).powi(2);
    let s_l = 1.0 + (0.015 * l_avg_minus_50_sq) / (20.0 + l_avg_minus_50_sq).sqrt();
    let s_c = 1.0 + 0.045 * c_avg_prime;
    let s_h = 1.0 + 0.015 * c_avg_prime * t;

    // Calculate R_T (rotation function)
    let delta_theta = 30.0 * (-((h_avg_prime - 275.0) / 25.0).powi(2)).exp();
    let c_avg_prime_pow7 = c_avg_prime.powi(7);
    let r_c = 2.0 * (c_avg_prime_pow7 / (c_avg_prime_pow7 + 6103515625.0_f64)).sqrt();
    let r_t = -r_c * (2.0 * delta_theta.to_radians()).sin();

    // Calculate final deltaE2000
    let term1 = delta_l_prime / (k_l * s_l);
    let term2 = delta_c_prime / (k_c * s_c);
    let term3 = delta_h_prime_big / (k_h * s_h);
    let term4 = r_t * (delta_c_prime / (k_c * s_c)) * (delta_h_prime_big / (k_h * s_h));

    (term1 * term1 + term2 * term2 + term3 * term3 + term4).sqrt()
}

/// Convert sRGB (0-255) to linear RGB
pub fn srgb_to_linear(value: u8) -> f64 {
    let v = value as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB to XYZ (D65)
pub fn linear_rgb_to_xyz(r: f64, g: f64, b: f64) -> [f64; 3] {
    [
        r * 0.4124564 + g * 0.3575761 + b * 0.1804375,
        r * 0.2126729 + g * 0.7151522 + b * 0.0721750,
        r * 0.0193339 + g * 0.1191920 + b * 0.9503041,
    ]
}

/// Convert XYZ (D65) to Lab
pub fn xyz_to_lab(xyz: [f64; 3]) -> [f64; 3] {
    // D65 white point
    let xn = 0.95047;
    let yn = 1.0;
    let zn = 1.08883;

    let fx = lab_f(xyz[0] / xn);
    let fy = lab_f(xyz[1] / yn);
    let fz = lab_f(xyz[2] / zn);

    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_f(t: f64) -> f64 {
    let delta: f64 = 6.0 / 29.0;
    if t > delta.powi(3) {
        t.powf(1.0 / 3.0)
    } else {
        t / (3.0 * delta * delta) + 4.0 / 29.0
    }
}

/// Convert sRGB to Lab
pub fn srgb_to_lab(r: u8, g: u8, b: u8) -> [f64; 3] {
    let lr = srgb_to_linear(r);
    let lg = srgb_to_linear(g);
    let lb = srgb_to_linear(b);
    let xyz = linear_rgb_to_xyz(lr, lg, lb);
    xyz_to_lab(xyz)
}

/// Compare two RGB pixel buffers and compute deltaE statistics
pub fn compare_rgb_buffers(reference: &[u8], result: &[u8]) -> DeltaEStats {
    assert_eq!(reference.len(), result.len());
    assert_eq!(reference.len() % 3, 0);

    let pixel_count = reference.len() / 3;
    let mut delta_es: Vec<f64> = Vec::with_capacity(pixel_count);

    for i in 0..pixel_count {
        let idx = i * 3;
        let lab_ref = srgb_to_lab(reference[idx], reference[idx + 1], reference[idx + 2]);
        let lab_res = srgb_to_lab(result[idx], result[idx + 1], result[idx + 2]);
        delta_es.push(delta_e_2000(lab_ref, lab_res));
    }

    delta_es.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mean: f64 = delta_es.iter().sum::<f64>() / delta_es.len() as f64;
    let max = *delta_es.last().unwrap_or(&0.0);
    let p95_idx = (delta_es.len() as f64 * 0.95) as usize;
    let p95 = delta_es.get(p95_idx).copied().unwrap_or(0.0);

    DeltaEStats {
        mean,
        max,
        p95,
        count: pixel_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_e_same_color() {
        let lab = [50.0, 25.0, -25.0];
        let de = delta_e_2000(lab, lab);
        assert!(de < 0.0001, "Same color should have deltaE ~0");
    }

    #[test]
    fn test_delta_e_different_colors() {
        let lab1 = [50.0, 0.0, 0.0];
        let lab2 = [51.0, 0.0, 0.0];
        let de = delta_e_2000(lab1, lab2);
        assert!(de > 0.0 && de < 2.0, "Small L difference: deltaE={}", de);
    }

    #[test]
    fn test_identical_buffers() {
        let buf = [255, 128, 64, 32, 16, 8];
        let stats = compare_rgb_buffers(&buf, &buf);
        assert!(stats.is_excellent());
        assert!(stats.mean < 0.0001);
    }
}
