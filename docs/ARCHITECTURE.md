# oxcms Architecture

## Overview

oxcms is a Rust color management system that aims to combine the best of existing CMS libraries:

| Library | What We Take |
|---------|--------------|
| **moxcms** | Rust safety, SIMD performance, modern API |
| **lcms2** | Full ICC v4.4 support, accuracy reference, test suite |
| **skcms** | Security hardening patterns, HDR support |
| **qcms** | Firefox battle-tested reliability |

## Current State

### Phase 1 Complete: Test Infrastructure

```
oxcms/
├── crates/
│   ├── oxcms-core/          # Thin wrapper over moxcms (to be replaced)
│   ├── cms-tests/           # 185 parity tests, all passing
│   └── skcms-sys/           # FFI bindings to skcms
├── external/
│   └── moxcms/              # Forked moxcms with ARM64 NEON fix
└── testdata/
    └── corpus/              # 121 ICC profiles for testing
```

**oxcms-core** currently wraps moxcms:
- `profile.rs` - Wraps `moxcms::ColorProfile`
- `transform.rs` - Wraps `moxcms::Transform*Executor`
- `error.rs` - Error types with `#[non_exhaustive]`

This wrapper provides a stable API surface while we build our own implementation.

## Target Architecture

### Phase 2+: Independent Implementation

```
oxcms/
├── crates/
│   ├── oxcms-core/              # Main CMS implementation
│   │   ├── src/
│   │   │   ├── lib.rs           # Public API
│   │   │   ├── error.rs         # Error types
│   │   │   │
│   │   │   ├── color/           # Color space primitives
│   │   │   │   ├── mod.rs
│   │   │   │   ├── xyz.rs       # CIE XYZ
│   │   │   │   ├── lab.rs       # CIELAB (L*a*b*)
│   │   │   │   ├── rgb.rs       # RGB primitives
│   │   │   │   └── white_point.rs
│   │   │   │
│   │   │   ├── math/            # Mathematical operations
│   │   │   │   ├── mod.rs
│   │   │   │   ├── matrix.rs    # 3x3 matrix ops
│   │   │   │   ├── gamma.rs     # Transfer functions
│   │   │   │   └── interpolation.rs
│   │   │   │
│   │   │   ├── icc/             # ICC profile parsing
│   │   │   │   ├── mod.rs
│   │   │   │   ├── header.rs    # 128-byte header
│   │   │   │   ├── tag_table.rs # Tag directory
│   │   │   │   ├── tags/        # Individual tag parsers
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── xyz.rs   # XYZ tags
│   │   │   │   │   ├── trc.rs   # TRC curves
│   │   │   │   │   ├── text.rs  # Text tags
│   │   │   │   │   └── lut.rs   # LUT tags
│   │   │   │   ├── v2.rs        # ICC v2 specifics
│   │   │   │   └── v4.rs        # ICC v4 specifics
│   │   │   │
│   │   │   ├── transform/       # Color transformations
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pipeline.rs  # Transform pipeline
│   │   │   │   ├── matrix_shaper.rs
│   │   │   │   ├── lut.rs       # LUT-based transforms
│   │   │   │   └── chromatic_adaptation.rs
│   │   │   │
│   │   │   ├── lut/             # LUT handling
│   │   │   │   ├── mod.rs
│   │   │   │   ├── lut1d.rs
│   │   │   │   ├── lut3d.rs
│   │   │   │   └── interpolation.rs
│   │   │   │
│   │   │   └── simd/            # SIMD acceleration
│   │   │       ├── mod.rs       # Feature detection + dispatch
│   │   │       ├── scalar.rs    # Scalar reference
│   │   │       ├── sse4.rs      # x86 SSE4.1
│   │   │       ├── avx2.rs      # x86 AVX2
│   │   │       └── neon.rs      # ARM NEON
│   │   │
│   │   └── Cargo.toml
│   │
│   ├── cms-tests/               # Parity test framework
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── accuracy.rs      # DeltaE measurement
│   │   │   ├── patterns.rs      # Test pattern generation
│   │   │   ├── reference.rs     # Reference impl wrappers
│   │   │   └── corpus.rs        # Test corpus management
│   │   └── tests/               # 20+ test files
│   │
│   └── skcms-sys/               # FFI to skcms (C++)
│
├── external/
│   └── moxcms/                  # Reference during development
│
├── testdata/
│   ├── corpus/                  # 121 ICC profiles
│   └── reference-outputs/       # lcms2 reference values
│
├── docs/
│   ├── ARCHITECTURE.md          # This file
│   └── MATH_DIFFERENCES.md      # Documented differences
│
├── plans/
│   ├── ROADMAP.md               # Implementation roadmap
│   └── IMPLEMENTATION_PLAN.md   # Detailed approach
│
└── tracking/
    └── TEST_STATUS.md           # Current test status
```

## Design Principles

### 1. lcms2 Output Compatibility

The primary goal is matching lcms2 output exactly:
- Same input → same output (within floating-point tolerance)
- All rendering intents produce identical results
- Edge cases handled identically

**Why lcms2?**
- Industry standard for color management
- Used by Adobe, GIMP, ImageMagick, and most print workflows
- Most complete ICC implementation
- Extensive test suite we can port

### 2. Layer-by-Layer Validation

Each layer is tested independently against reference:

```
┌─────────────────────────────────────────┐
│          Public API (Transform)         │  ← Test: end-to-end parity
├─────────────────────────────────────────┤
│         Transform Pipeline              │  ← Test: pipeline stages
├─────────────────────────────────────────┤
│    Matrix-Shaper  │   LUT-Based         │  ← Test: transform types
├───────────────────┼─────────────────────┤
│         ICC Profile Parsing             │  ← Test: parsed values
├─────────────────────────────────────────┤
│    Color Math  │  Interpolation         │  ← Test: math operations
└─────────────────────────────────────────┘
```

If end-to-end test fails, we know exactly which layer diverged.

### 3. Scalar-First Development

1. Implement scalar (non-SIMD) version first
2. Validate scalar against lcms2
3. Add SIMD acceleration
4. Validate SIMD matches scalar exactly

The scalar implementation is the reference. SIMD must produce identical output.

### 4. All Differences Documented

Any deviation from lcms2 must be:
1. Detected by parity tests
2. Documented in MATH_DIFFERENCES.md
3. Justified with technical reasoning
4. Accepted explicitly (not silently tolerated)

## Transform Pipeline

### Matrix-Shaper Profile (90% of use cases)

```
Input RGB
    │
    ▼
┌─────────────────┐
│  Input TRC      │  ← Linearize (gamma decode)
│  (Tone Curve)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  RGB → XYZ      │  ← 3x3 matrix from colorant tags
│  Matrix         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Chromatic      │  ← Bradford adaptation D65→D50
│  Adaptation     │     (if white points differ)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  XYZ → RGB      │  ← Inverse of destination matrix
│  Matrix         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Output TRC     │  ← Re-apply gamma (encode)
│  (Tone Curve)   │
└────────┬────────┘
         │
         ▼
   Output RGB
```

### LUT-Based Profile

```
Input RGB
    │
    ▼
┌─────────────────┐
│  A Curves       │  ← Pre-process (optional)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  3D CLUT        │  ← Color lookup table
│  (Tetrahedral)  │     with interpolation
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  M Curves       │  ← Mid-process (optional)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Matrix         │  ← Linear transform (optional)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  B Curves       │  ← Post-process
└────────┬────────┘
         │
         ▼
   Output RGB
```

## SIMD Strategy

### Feature Detection

```rust
// Runtime detection
#[cfg(target_arch = "x86_64")]
fn select_impl() -> TransformImpl {
    if is_x86_feature_detected!("avx2") {
        TransformImpl::Avx2
    } else if is_x86_feature_detected!("sse4.1") {
        TransformImpl::Sse4
    } else {
        TransformImpl::Scalar
    }
}
```

### SIMD Invariants

1. **SIMD must match scalar exactly** - No "close enough"
2. **Scalar always available** - No platform without working impl
3. **Test all paths** - CI runs scalar, SSE, AVX2, NEON
4. **Document any precision differences** - If SIMD uses different precision, document why

### Known SIMD Gotchas

From moxcms ARM64 bug fix:
- **Copy-paste errors in multi-pixel processing** - NEON processes 4 pixels at once, easy to use wrong register
- **Fixed-point vs floating-point** - Different precision characteristics
- **Lane ordering** - SIMD lanes may have different ordering per architecture

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("Profile parse error: {0}")]
    ProfileParse(String),

    #[error("Invalid profile: {0}")]
    InvalidProfile(String),

    #[error("Transform error: {0}")]
    Transform(String),

    // ... more variants
}
```

**`#[non_exhaustive]`** allows adding variants without breaking downstream.

### Panic Policy

- **Never panic on malformed input** - Return Error instead
- **Never panic in transforms** - Even on buffer size mismatch
- **Panics only for internal invariant violations** - Debug assertions

## Testing Strategy

### Test Categories

| Category | Purpose | Count |
|----------|---------|-------|
| Unit tests | Individual function behavior | ~50 |
| Parity tests | Match lcms2/moxcms/qcms output | ~100 |
| Corpus tests | Parse 121 real ICC profiles | ~30 |
| Fuzz tests | Security, malformed input | TBD |

### Reference Implementations

```rust
// cms-tests/src/reference.rs
pub fn transform_lcms2_srgb(input: &[u8]) -> Result<Vec<u8>>;
pub fn transform_moxcms_srgb(input: &[u8]) -> Result<Vec<u8>>;
pub fn transform_qcms_srgb(input: &[u8]) -> Result<Vec<u8>>;
pub fn transform_skcms_srgb(input: &[u8]) -> Result<Vec<u8>>;
```

### DeltaE Measurement

All parity tests use DeltaE2000:
- `< 0.1` - Invisible difference
- `0.1-0.5` - Barely visible to trained observers
- `0.5-1.0` - Threshold of perception
- `> 1.0` - **Test failure** (unless documented exception)

## Dependencies

### Runtime Dependencies

| Crate | Purpose |
|-------|---------|
| `thiserror` | Error derive macro |
| `bytemuck` | Safe transmutes for SIMD |

### Dev/Test Dependencies

| Crate | Purpose |
|-------|---------|
| `lcms2` | Reference implementation |
| `qcms` | Firefox CMS comparison |
| `moxcms` | Performance baseline |
| `palette` | Color math validation |
| `criterion` | Benchmarking |

## Performance Targets

| Operation | Target | Reference |
|-----------|--------|-----------|
| sRGB→sRGB 1MP | < 1ms | moxcms baseline |
| sRGB→P3 1MP | < 2ms | moxcms baseline |
| ICC parse | < 100μs | moxcms baseline |

We aim to match or exceed moxcms performance, which is already 3x+ faster than lcms2.

## Security Considerations

### Threat Model

ICC profiles are untrusted input (can come from images, PDFs, etc.):
- Malformed headers
- Invalid tag offsets (buffer overruns)
- Huge LUT sizes (DoS)
- Infinite loops in curve evaluation

### Mitigations

1. **Bounds checking** - All array accesses checked
2. **Size limits** - Maximum LUT size, maximum profile size
3. **Fuzzing** - OSS-Fuzz integration planned
4. **No unsafe in parsing** - Only in validated SIMD paths
