# oxcms Implementation Plan

This document details the concrete implementation approach for building oxcms as an independent CMS.

## Strategy

### Approach: Incremental Replacement

We will incrementally replace moxcms components with our own implementation:

1. **Keep moxcms as reference** - Always available for comparison
2. **Build new modules alongside** - Don't break existing functionality
3. **Switch module-by-module** - Feature flags to toggle between implementations
4. **Validate at each step** - Parity tests must pass before proceeding

### Why Not Fork moxcms Directly?

1. **Understanding** - Building from scratch ensures deep understanding
2. **Clean API** - Design API for our specific needs
3. **Documentation** - Every line has clear rationale
4. **Auditability** - Fresh code is easier to audit than complex fork

### Feature Flags

```toml
[features]
default = ["use-moxcms"]  # Start with moxcms backend
use-moxcms = ["moxcms"]   # Use moxcms for transforms
native-math = []          # Use our math implementation
native-icc = []           # Use our ICC parser
native-transform = []     # Use our transform pipeline
simd = ["native-transform"]  # Enable SIMD (requires native)
```

## Implementation Order

### Phase 2.1: Core Color Math

**Goal**: Foundation math that doesn't depend on moxcms.

#### Step 1: Matrix Operations

**File**: `src/math/matrix.rs`

```rust
/// 3x3 matrix for RGB↔XYZ transforms
#[derive(Debug, Clone, Copy)]
pub struct Matrix3x3 {
    pub m: [[f64; 3]; 3],
}

impl Matrix3x3 {
    pub fn multiply(&self, v: [f64; 3]) -> [f64; 3];
    pub fn inverse(&self) -> Option<Self>;
    pub fn transpose(&self) -> Self;
    pub fn multiply_matrix(&self, other: &Self) -> Self;
}
```

**Tests**:
- Matrix inversion matches lcms2's `_cmsMAT3inverse`
- Matrix multiplication matches lcms2
- Round-trip (M × M⁻¹ = I) within tolerance

**Reference**: lcms2 `cmsmtrx.c`

#### Step 2: White Points

**File**: `src/color/white_point.rs`

```rust
/// CIE Standard Illuminant white points
pub mod white_points {
    pub const D50: Xyz = Xyz { x: 0.9642, y: 1.0, z: 0.8251 };
    pub const D65: Xyz = Xyz { x: 0.9505, y: 1.0, z: 1.0890 };
    pub const D60: Xyz = Xyz { x: 0.9523, y: 1.0, z: 1.0084 };
    pub const DCI: Xyz = Xyz { x: 0.8940, y: 1.0, z: 0.9544 };
}
```

**Tests**: Values match lcms2 `cmspcs.c` and ICC spec.

**Reference**: ICC.1:2022 Table 14

#### Step 3: Chromatic Adaptation

**File**: `src/math/chromatic_adaptation.rs`

```rust
/// Bradford chromatic adaptation matrix
pub fn bradford_adaptation(src_white: Xyz, dst_white: Xyz) -> Matrix3x3;

/// Adapt XYZ from one white point to another
pub fn adapt_xyz(xyz: Xyz, src_white: Xyz, dst_white: Xyz) -> Xyz;
```

**Tests**:
- D65→D50 adaptation matches lcms2
- Round-trip D65→D50→D65 within tolerance
- Identity when src_white == dst_white

**Reference**: lcms2 `cmscam97.c`, Bradford matrix from Lindbloom

#### Step 4: sRGB Gamma

**File**: `src/math/gamma.rs`

```rust
/// sRGB gamma encode (linear → encoded)
pub fn srgb_gamma_encode(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB gamma decode (encoded → linear)
pub fn srgb_gamma_decode(encoded: f64) -> f64 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// Parametric curve type (ICC signature 'para')
pub struct ParametricCurve {
    pub function_type: u16,
    pub params: [f64; 7],
}
```

**Tests**:
- sRGB round-trip for all 256 values
- Compare to lcms2 `_cmsBuildParametricToneCurve`
- Parametric curve types 0-4 match ICC spec

**Reference**: IEC 61966-2-1, lcms2 `cmsgamma.c`

### Phase 2.2: ICC Profile Parsing

**Goal**: Parse ICC profiles without moxcms.

#### Step 1: Header Parsing

**File**: `src/icc/header.rs`

```rust
/// 128-byte ICC profile header
#[repr(C)]
pub struct IccHeader {
    pub size: u32,
    pub cmm_type: u32,
    pub version: ProfileVersion,
    pub device_class: ProfileClass,
    pub color_space: ColorSpace,
    pub pcs: ColorSpace,
    pub creation_date: DateTime,
    pub signature: u32,  // 'acsp'
    pub platform: u32,
    pub flags: u32,
    pub manufacturer: u32,
    pub model: u32,
    pub attributes: u64,
    pub rendering_intent: RenderingIntent,
    pub illuminant: XyzNumber,
    pub creator: u32,
    pub profile_id: [u8; 16],
    pub reserved: [u8; 28],
}

impl IccHeader {
    pub fn parse(data: &[u8]) -> Result<Self, IccError>;
    pub fn validate(&self) -> Result<(), IccError>;
}
```

**Tests**:
- Parse all 121 corpus profiles without error
- Header values match moxcms parsed values
- Reject malformed headers (wrong signature, impossible sizes)

**Reference**: ICC.1:2022 Section 7.2

#### Step 2: Tag Table

**File**: `src/icc/tag_table.rs`

```rust
/// Tag table entry
pub struct TagEntry {
    pub signature: TagSignature,
    pub offset: u32,
    pub size: u32,
}

/// Parse tag table from profile
pub fn parse_tag_table(data: &[u8], header: &IccHeader) -> Result<Vec<TagEntry>, IccError>;

/// Get tag data by signature
pub fn get_tag_data<'a>(data: &'a [u8], entry: &TagEntry) -> Result<&'a [u8], IccError>;
```

**Tests**:
- Parse tag tables from corpus
- Detect overlapping tags
- Reject out-of-bounds offsets

**Reference**: ICC.1:2022 Section 7.3

#### Step 3: XYZ Tags

**File**: `src/icc/tags/xyz.rs`

```rust
/// Parse XYZ tag (rXYZ, gXYZ, bXYZ, wtpt)
pub fn parse_xyz_tag(data: &[u8]) -> Result<Xyz, IccError>;
```

**Tests**:
- Parse sRGB colorant tags
- Values match lcms2 `cmsReadTag`

**Reference**: ICC.1:2022 Section 10.31

#### Step 4: TRC Tags

**File**: `src/icc/tags/trc.rs`

```rust
/// Tone Reproduction Curve
pub enum ToneCurve {
    /// Single gamma value
    Gamma(f64),
    /// Lookup table
    Lut(Vec<u16>),
    /// Parametric curve
    Parametric(ParametricCurve),
}

/// Parse TRC tag (rTRC, gTRC, bTRC)
pub fn parse_trc_tag(data: &[u8]) -> Result<ToneCurve, IccError>;
```

**Tests**:
- Parse sRGB TRC (parametric type 3)
- Parse legacy profiles with LUT TRCs
- Gamma-only TRCs

**Reference**: ICC.1:2022 Section 10.5, 10.18

#### Step 5: LUT Tags (A2B, B2A)

**File**: `src/icc/tags/lut.rs`

```rust
/// Multi-dimensional LUT
pub struct Lut3D {
    pub grid_points: [u8; 3],
    pub data: Vec<u16>,
}

/// Parse A2B or B2A tag
pub fn parse_lut_tag(data: &[u8]) -> Result<LutTag, IccError>;
```

**Tests**:
- Parse CMYK profile LUTs
- Parse v4 mAB/mBA tags
- Verify LUT dimensions and data

**Reference**: ICC.1:2022 Sections 10.10, 10.11, 10.14, 10.15

### Phase 2.3: Matrix-Shaper Transforms

**Goal**: Implement sRGB↔sRGB and sRGB↔P3 transforms.

#### Step 1: Transform Pipeline

**File**: `src/transform/pipeline.rs`

```rust
/// A transform is a sequence of stages
pub struct TransformPipeline {
    stages: Vec<Box<dyn Stage>>,
}

pub trait Stage: Send + Sync {
    fn process(&self, input: &[f64], output: &mut [f64]);
}
```

#### Step 2: Matrix-Shaper Profile

**File**: `src/transform/matrix_shaper.rs`

```rust
/// Build transform for matrix-shaper profile
pub fn build_matrix_shaper_transform(
    src: &Profile,
    dst: &Profile,
    intent: RenderingIntent,
) -> Result<TransformPipeline, Error>;
```

Pipeline stages:
1. Input TRC (gamma decode)
2. RGB→XYZ matrix
3. Chromatic adaptation (if needed)
4. XYZ→RGB matrix
5. Output TRC (gamma encode)

**Tests**:
- sRGB→sRGB produces identity (max diff = 0)
- sRGB→P3 matches lcms2 output exactly
- P3→sRGB matches lcms2 output exactly

#### Step 3: 8-bit Fast Path

**File**: `src/transform/fast_8bit.rs`

```rust
/// Pre-computed LUT for 8-bit transforms
pub struct FastTransform8Bit {
    /// Combined input TRC + matrix + output TRC
    lut: [[u8; 256]; 3],  // For each output channel
    // ... or 3D LUT for more accuracy
}
```

**Tests**:
- Fast path matches full-precision path
- Performance within 2x of moxcms

### Phase 2.4: LUT-Based Transforms

**Goal**: Support CMYK profiles and device links.

#### Step 1: 3D LUT Interpolation

**File**: `src/lut/interpolation.rs`

```rust
/// Trilinear interpolation in 3D LUT
pub fn trilinear_interp(lut: &Lut3D, rgb: [f64; 3]) -> [f64; 3];

/// Tetrahedral interpolation (more accurate)
pub fn tetrahedral_interp(lut: &Lut3D, rgb: [f64; 3]) -> [f64; 3];
```

**Tests**:
- Interpolation matches lcms2 `_cmsTrilinearInterp16`
- Edge cases: corners, edges, faces of cube

**Reference**: lcms2 `cmsintrp.c`

#### Step 2: LUT Transform Pipeline

**File**: `src/transform/lut.rs`

```rust
/// Build transform for LUT-based profile
pub fn build_lut_transform(
    src: &Profile,
    dst: &Profile,
    intent: RenderingIntent,
) -> Result<TransformPipeline, Error>;
```

**Tests**:
- CMYK→sRGB matches lcms2
- sRGB→CMYK matches lcms2
- DeviceLink profiles work

### Phase 2.5: SIMD Optimization

**Goal**: Match moxcms performance.

#### Step 1: SIMD Trait

**File**: `src/simd/mod.rs`

```rust
/// SIMD implementation trait
pub trait SimdImpl {
    fn transform_8bit(&self, src: &[u8], dst: &mut [u8]);
}

/// Runtime dispatch
pub fn select_simd_impl() -> Box<dyn SimdImpl> {
    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        return Box::new(Avx2Impl::new());
    }
    // ... etc
    Box::new(ScalarImpl::new())
}
```

#### Step 2: SSE4 Implementation

**File**: `src/simd/sse4.rs`

Process 4 pixels at a time using 128-bit registers.

**Tests**:
- Output matches scalar exactly
- All test patterns pass

#### Step 3: AVX2 Implementation

**File**: `src/simd/avx2.rs`

Process 8 pixels at a time using 256-bit registers.

**Tests**:
- Output matches scalar exactly
- All test patterns pass

#### Step 4: NEON Implementation

**File**: `src/simd/neon.rs`

Process 4 pixels at a time using 128-bit NEON registers.

**CRITICAL**: Learn from moxcms ARM64 bug:
- Test every pixel position independently
- Compare SIMD output to scalar for every test
- No copy-paste between pixel processing blocks

**Tests**:
- Output matches scalar exactly for ALL patterns
- Specific test for second-pixel blue channel (the bug we fixed)

## Validation Strategy

### Level 1: Unit Tests

Each function tested against known values from lcms2.

```rust
#[test]
fn test_srgb_gamma_decode() {
    // Values from lcms2
    assert_eq!(srgb_gamma_decode(0.0), 0.0);
    assert_eq!(srgb_gamma_decode(0.5), 0.214041);
    assert_eq!(srgb_gamma_decode(1.0), 1.0);
}
```

### Level 2: Integration Tests

Full transforms compared to lcms2 output.

```rust
#[test]
fn test_srgb_to_p3_matches_lcms2() {
    let input = generate_test_pattern();
    let our_output = oxcms_transform(&input);
    let lcms2_output = lcms2_transform(&input);
    assert_eq!(our_output, lcms2_output);
}
```

### Level 3: Corpus Tests

Parse and transform using all 121 corpus profiles.

### Level 4: Fuzz Tests

Random/malformed input to find edge cases.

## Module Checklist

### Core Math
- [ ] `math/matrix.rs` - 3x3 matrix operations
- [ ] `math/gamma.rs` - Gamma/TRC functions
- [ ] `math/chromatic_adaptation.rs` - Bradford adaptation
- [ ] `math/interpolation.rs` - LUT interpolation

### Color Types
- [ ] `color/xyz.rs` - CIE XYZ
- [ ] `color/lab.rs` - CIELAB
- [ ] `color/rgb.rs` - RGB primitives
- [ ] `color/white_point.rs` - Standard illuminants

### ICC Parsing
- [ ] `icc/header.rs` - Profile header
- [ ] `icc/tag_table.rs` - Tag directory
- [ ] `icc/tags/xyz.rs` - XYZ tags
- [ ] `icc/tags/trc.rs` - TRC curves
- [ ] `icc/tags/text.rs` - Text/desc tags
- [ ] `icc/tags/lut.rs` - LUT tags
- [ ] `icc/profile.rs` - Complete profile

### Transform Pipeline
- [ ] `transform/pipeline.rs` - Stage abstraction
- [ ] `transform/matrix_shaper.rs` - Matrix profiles
- [ ] `transform/lut.rs` - LUT profiles
- [ ] `transform/chromatic_adaptation.rs` - White point adaptation

### SIMD
- [ ] `simd/scalar.rs` - Reference implementation
- [ ] `simd/sse4.rs` - x86 SSE4
- [ ] `simd/avx2.rs` - x86 AVX2
- [ ] `simd/neon.rs` - ARM NEON

## Dependencies

### Required for Implementation
- None (pure Rust, no external deps for core)

### Required for Validation
- `lcms2` - Reference outputs
- `moxcms` - Performance baseline
- `qcms` - Additional reference

### Optional
- `bytemuck` - Safe transmutes for SIMD
- `rayon` - Parallel processing (large images)

## Risk Mitigation

### Risk: Floating-Point Precision

**Mitigation**:
- Use f64 internally, convert to f32/f64 at boundaries
- Document precision requirements for each operation
- Test with edge cases (very small, very large values)

### Risk: SIMD Bugs

**Mitigation**:
- Scalar implementation is the reference
- Every SIMD path tested against scalar
- Multi-pixel processing tested for each pixel position
- CI tests all SIMD variants

### Risk: ICC Spec Ambiguity

**Mitigation**:
- lcms2 behavior is the reference
- Document any spec interpretations
- Test against real-world profiles

## Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| lcms2 parity | 100% | All parity tests pass |
| Performance | ≥ moxcms | Benchmark suite |
| Test coverage | > 80% | cargo-tarpaulin |
| Zero unsafe (core) | Yes | `#![forbid(unsafe_code)]` on non-SIMD |
