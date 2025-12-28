# Codec Integration Guide

This guide shows how to integrate oxcms into image codecs that need XYB color space support (JPEG XL) or CMYK support (JPEG, TIFF, PDF).

## Table of Contents

- [XYB Color Space (JPEG XL)](#xyb-color-space-jpeg-xl)
- [CMYK Support](#cmyk-support)
- [Performance Considerations](#performance-considerations)

---

## XYB Color Space (JPEG XL)

XYB is a perceptually uniform color space used internally by JPEG XL. It's an LMS-based model with cube root gamma, designed for optimal compression of natural images.

### XYB Characteristics

| Property | Description |
|----------|-------------|
| **X channel** | L-M opponent (red-green difference) |
| **Y channel** | Luminance-like ((L+M)/2) |
| **B channel** | S-M (blue channel) |
| **Gamma** | Cube root (γ = 3) |
| **Bias** | 0.00379... added before cube root |

### XYB Implementation

XYB is **not** an ICC profile color space - it must be handled outside the ICC pipeline. Here's a complete implementation:

```rust
/// XYB color space constants
pub mod xyb {
    /// Bias added before cube root in forward transform
    pub const BIAS: f64 = 0.003_793_073_255_275_449_3;

    /// Cube root of BIAS, subtracted after cube root
    pub const BIAS_CBRT: f64 = 0.155_954_200_549_248_63;

    /// XYB color value
    #[derive(Debug, Clone, Copy)]
    pub struct Xyb {
        pub x: f64,
        pub y: f64,
        pub b: f64,
    }

    /// Cube root preserving sign
    #[inline]
    fn cbrt(x: f64) -> f64 {
        if x >= 0.0 { x.cbrt() } else { -(-x).cbrt() }
    }

    /// Convert linear RGB to XYB
    ///
    /// Input: Linear RGB (not gamma-encoded), range [0, 1]
    /// Output: XYB, approximate ranges X:[-0.05,0.05], Y:[0,0.85], B:[-0.45,0.45]
    pub fn linear_rgb_to_xyb(r: f64, g: f64, b: f64) -> Xyb {
        // RGB to LMS matrix
        let l_linear = 0.3 * r + 0.622 * g + 0.078 * b;
        let m_linear = 0.23 * r + 0.692 * g + 0.078 * b;
        let s_linear = 0.243_422_689_245_478_2 * r
            + 0.204_767_444_244_968_2 * g
            + 0.551_809_866_509_553_5 * b;

        // Apply bias and cube root
        let l_gamma = cbrt(l_linear + BIAS) - BIAS_CBRT;
        let m_gamma = cbrt(m_linear + BIAS) - BIAS_CBRT;
        let s_gamma = cbrt(s_linear + BIAS) - BIAS_CBRT;

        // LMS to XYB
        Xyb {
            x: (l_gamma - m_gamma) * 0.5,
            y: (l_gamma + m_gamma) * 0.5,
            b: s_gamma - m_gamma,
        }
    }

    /// Convert XYB to linear RGB
    pub fn xyb_to_linear_rgb(xyb: &Xyb) -> (f64, f64, f64) {
        // XYB to LMS
        let l_gamma = xyb.x + xyb.y + BIAS_CBRT;
        let m_gamma = -xyb.x + xyb.y + BIAS_CBRT;
        let s_gamma = -xyb.x + xyb.y + xyb.b + BIAS_CBRT;

        // Apply cubic (inverse of cube root)
        let l_linear = l_gamma.powi(3) - BIAS;
        let m_linear = m_gamma.powi(3) - BIAS;
        let s_linear = s_gamma.powi(3) - BIAS;

        // LMS to RGB matrix (inverse of forward matrix)
        let r = 11.031566901960783 * l_linear
            - 9.866943921568629 * m_linear
            - 0.16462299647058826 * s_linear;
        let g = -3.254147380392157 * l_linear
            + 4.418770392156863 * m_linear
            - 0.16462299647058826 * s_linear;
        let b = -3.6588512862745097 * l_linear
            + 2.7129230470588235 * m_linear
            + 1.9459282392156863 * s_linear;

        (r, g, b)
    }
}
```

### JPEG XL Decoder Integration

For a JPEG XL decoder, the typical pipeline is:

```
JPEG XL bitstream
       ↓
   [Entropy decode]
       ↓
   XYB coefficients
       ↓
   [Inverse DCT / reconstruction]
       ↓
   XYB pixels
       ↓
   [xyb_to_linear_rgb]          ← oxcms XYB conversion
       ↓
   Linear sRGB
       ↓
   [ICC profile transform]      ← oxcms ICC handling
       ↓
   Output color space (e.g., Display P3)
       ↓
   [Gamma encode]
       ↓
   Final output
```

### Example: JXL Decoder with ICC

```rust
use oxcms_core::{ColorProfile, Layout, TransformOptions};

fn decode_jxl_frame(
    xyb_data: &[f32],           // XYB pixels from decoder
    embedded_icc: Option<&[u8]>, // ICC profile from JXL container
    width: usize,
    height: usize,
) -> Vec<u8> {
    // Step 1: Convert XYB to linear RGB
    let mut linear_rgb: Vec<f64> = Vec::with_capacity(width * height * 3);
    for chunk in xyb_data.chunks_exact(3) {
        let xyb = xyb::Xyb {
            x: chunk[0] as f64,
            y: chunk[1] as f64,
            b: chunk[2] as f64,
        };
        let (r, g, b) = xyb::xyb_to_linear_rgb(&xyb);
        linear_rgb.extend_from_slice(&[r, g, b]);
    }

    // Step 2: Determine source profile
    // JXL default is linear sRGB primaries
    let source_profile = ColorProfile::new_linear_srgb();

    // Step 3: Determine output profile
    let output_profile = if let Some(icc_data) = embedded_icc {
        ColorProfile::from_icc(icc_data).unwrap_or_else(|_| ColorProfile::new_srgb())
    } else {
        ColorProfile::new_srgb()  // Default output
    };

    // Step 4: Create ICC transform
    let transform = source_profile.create_transform_f32(
        Layout::Rgb,
        &output_profile,
        Layout::Rgb,
        TransformOptions::default(),
    ).expect("Failed to create transform");

    // Step 5: Apply ICC transform and encode to sRGB gamma
    let linear_f32: Vec<f32> = linear_rgb.iter().map(|&v| v as f32).collect();
    let mut output_f32 = vec![0.0f32; linear_f32.len()];
    transform.transform_f32(&linear_f32, &mut output_f32).unwrap();

    // Step 6: Apply output gamma and convert to u8
    output_f32.iter()
        .map(|&v| (srgb_gamma_encode(v.clamp(0.0, 1.0)) * 255.0).round() as u8)
        .collect()
}

fn srgb_gamma_encode(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}
```

### XYB Value Ranges

For sRGB gamut inputs:

| Channel | Min | Max | Notes |
|---------|-----|-----|-------|
| X | -0.05 | +0.05 | Red-green opponent |
| Y | 0.0 | 0.85 | Luminance-like |
| B | -0.45 | +0.45 | Blue channel |

---

## CMYK Support

CMYK is used in print workflows and embedded in JPEG, TIFF, and PDF files. oxcms supports CMYK via LUT-based ICC profile transforms.

### CMYK Profile Types

| Profile Type | Tags | Use Case |
|--------------|------|----------|
| **A2B only** | A2B0/1/2 | Device → PCS (for display) |
| **B2A only** | B2A0/1/2 | PCS → Device (for printing) |
| **Both** | A2B + B2A | Full round-trip support |

### CMYK Transform Pipeline

```
CMYK pixels (from JPEG/TIFF)
       ↓
   [Source A2B LUT]     ← Device → PCS (Lab or XYZ)
       ↓
   Profile Connection Space (PCS)
       ↓
   [Destination B2A LUT] ← PCS → Device  (or matrix-shaper for RGB)
       ↓
   Output pixels (RGB for display, CMYK for print)
```

### Example: CMYK JPEG Decoder

```rust
use oxcms_core::icc::IccProfile;
use oxcms_core::pipeline::{Pipeline, TransformContext, TransformFlags};
use oxcms_core::icc::header::RenderingIntent;

fn decode_cmyk_jpeg(
    cmyk_data: &[u8],           // CMYK pixels from JPEG decoder
    embedded_icc: Option<&[u8]>, // ICC profile from APP2 marker
    width: usize,
    height: usize,
    inverted: bool,             // Adobe APP14 marker indicates inverted CMYK
) -> Vec<u8> {
    // Step 1: Handle CMYK inversion (Adobe JPEGs store inverted CMYK)
    let cmyk_normalized: Vec<u8> = if inverted {
        cmyk_data.iter().map(|&v| 255 - v).collect()
    } else {
        cmyk_data.to_vec()
    };

    // Step 2: Parse source CMYK profile
    let source_profile = if let Some(icc_data) = embedded_icc {
        IccProfile::parse(icc_data).ok()
    } else {
        None
    };

    // Step 3: Load destination profile (sRGB for display)
    let srgb_bytes = include_bytes!("sRGB.icc");
    let dest_profile = IccProfile::parse(srgb_bytes).expect("Failed to parse sRGB");

    // Step 4: Create transform pipeline
    let ctx = TransformContext {
        intent: RenderingIntent::Perceptual,
        flags: TransformFlags {
            black_point_compensation: true,
            clamp_output: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let pipeline = if let Some(ref src) = source_profile {
        Pipeline::from_profiles(src, &dest_profile, &ctx)
            .expect("Failed to create CMYK pipeline")
    } else {
        // Fallback: Use default FOGRA39 or similar
        panic!("CMYK without ICC profile not supported - need fallback profile");
    };

    // Step 5: Transform CMYK → RGB
    let mut rgb_output = vec![0u8; width * height * 3];

    for (cmyk_chunk, rgb_chunk) in cmyk_normalized.chunks_exact(4)
        .zip(rgb_output.chunks_exact_mut(3))
    {
        let cmyk = [
            cmyk_chunk[0] as f64 / 255.0,
            cmyk_chunk[1] as f64 / 255.0,
            cmyk_chunk[2] as f64 / 255.0,
            cmyk_chunk[3] as f64 / 255.0,
        ];

        let rgb = pipeline.transform_cmyk_to_rgb(cmyk);

        rgb_chunk[0] = (rgb[0] * 255.0).round().clamp(0.0, 255.0) as u8;
        rgb_chunk[1] = (rgb[1] * 255.0).round().clamp(0.0, 255.0) as u8;
        rgb_chunk[2] = (rgb[2] * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    rgb_output
}
```

### Detecting Inverted CMYK in JPEG

Adobe Photoshop saves CMYK JPEGs with inverted values. Detect via APP14 marker:

```rust
/// Check if JPEG has Adobe APP14 marker indicating inverted CMYK
fn is_adobe_inverted_cmyk(jpeg_data: &[u8]) -> bool {
    // Look for APP14 marker (0xFF 0xEE)
    let mut i = 0;
    while i + 1 < jpeg_data.len() {
        if jpeg_data[i] == 0xFF {
            let marker = jpeg_data[i + 1];
            if marker == 0xEE {  // APP14
                // Adobe marker structure:
                // - Length (2 bytes)
                // - "Adobe" (5 bytes)
                // - Version (2 bytes)
                // - Flags0 (2 bytes)
                // - Flags1 (2 bytes)
                // - Color transform (1 byte): 0=Unknown, 1=YCbCr, 2=YCCK
                if i + 13 < jpeg_data.len() && &jpeg_data[i+4..i+9] == b"Adobe" {
                    let transform = jpeg_data[i + 13];
                    // transform=2 means YCCK (inverted CMYK)
                    return transform == 2;
                }
            }
            // Skip segment
            if marker >= 0xE0 && marker <= 0xEF {
                if i + 3 < jpeg_data.len() {
                    let len = u16::from_be_bytes([jpeg_data[i+2], jpeg_data[i+3]]) as usize;
                    i += 2 + len;
                    continue;
                }
            }
        }
        i += 1;
    }
    false
}
```

### Black Point Compensation

For print workflows, enable black point compensation to prevent crushing of dark tones:

```rust
use oxcms_core::pipeline::bpc::{BpcParams, detect_black_point};

fn transform_with_bpc(
    src_profile: &IccProfile,
    dst_profile: &IccProfile,
    xyz_values: &mut [[f64; 3]],
) {
    // Detect black points
    let src_bp = detect_black_point(src_profile, None);
    let dst_bp = detect_black_point(dst_profile, None);

    // Calculate BPC parameters
    if let (Some(sbp), Some(dbp)) = (src_bp, dst_bp) {
        if let Some(bpc) = BpcParams::calculate(sbp, dbp) {
            // Apply BPC to XYZ values in the PCS
            for xyz in xyz_values {
                *xyz = bpc.apply(*xyz);
            }
        }
    }
}
```

### CMYK Test Profiles Available

| Profile | Path | Description |
|---------|------|-------------|
| FOGRA39 | `testdata/profiles/skcms/misc/Coated_FOGRA39_CMYK.icc` | Standard European coated paper |
| ps_cmyk_min | `testdata/profiles/qcms/ps_cmyk_min.icc` | Minimal CMYK profile |
| test1.icc | `testdata/profiles/lcms2/test1.icc` | LittleCMS CMYK test |
| test2.icc | `testdata/profiles/lcms2/plugins/test2.icc` | LittleCMS CMYK test |

---

## Performance Considerations

### Batch Processing

For large images, use batch processing APIs:

```rust
use oxcms_core::simd::{transform_rgb8_batch, active_features};

fn process_large_image(src: &[u8], dst: &mut [u8], transform: &Transform) {
    println!("Using SIMD: {}", active_features());

    // Process in 64KB chunks for cache efficiency
    const CHUNK_SIZE: usize = 65536;

    for (src_chunk, dst_chunk) in src.chunks(CHUNK_SIZE)
        .zip(dst.chunks_mut(CHUNK_SIZE))
    {
        transform.transform(src_chunk, dst_chunk).unwrap();
    }
}
```

### LUT Caching

For repeated transforms with the same profiles, cache the pipeline:

```rust
use std::collections::HashMap;
use std::sync::Arc;

struct TransformCache {
    cache: HashMap<(ProfileId, ProfileId), Arc<Pipeline>>,
}

impl TransformCache {
    fn get_or_create(
        &mut self,
        src: &IccProfile,
        dst: &IccProfile,
        ctx: &TransformContext,
    ) -> Arc<Pipeline> {
        let key = (src.id(), dst.id());
        self.cache.entry(key)
            .or_insert_with(|| {
                Arc::new(Pipeline::from_profiles(src, dst, ctx).unwrap())
            })
            .clone()
    }
}
```

### Thread Safety

All oxcms transforms are thread-safe (Sync + Send). Process image tiles in parallel:

```rust
use rayon::prelude::*;

fn parallel_transform(
    src: &[u8],
    dst: &mut [u8],
    transform: &Transform,
    tile_size: usize,
) {
    let src_tiles: Vec<_> = src.chunks(tile_size).collect();
    let dst_tiles: Vec<_> = dst.chunks_mut(tile_size).collect();

    src_tiles.into_par_iter()
        .zip(dst_tiles)
        .for_each(|(src_tile, dst_tile)| {
            transform.transform(src_tile, dst_tile).unwrap();
        });
}
```

---

## Browser Considerations

### Chrome (skcms)
- Uses skcms for all ICC transforms
- CMYK JPEGs: Converts to sRGB using embedded profile
- No spot color support

### Firefox (qcms)
- Uses qcms (Rust) for ICC transforms
- CMYK handling similar to Chrome
- Limited to standard ICC v2/v4 profiles

### Safari
- Uses ColorSync (macOS) / system CMS
- Generally better CMYK support than Chrome/Firefox
- Respects embedded ICC profiles

### Recommendation
Always embed ICC profiles in CMYK images. Without profiles, browsers produce inconsistent results.

---

## See Also

- [ARCHITECTURE.md](ARCHITECTURE.md) - Overall design
- [MATH_DIFFERENCES.md](MATH_DIFFERENCES.md) - CMS comparison results
- XYB reference: https://facelessuser.github.io/coloraide/colors/xyb/
- ICC specification: https://www.color.org/specification/ICC.1-2022-05.pdf
