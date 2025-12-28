# oxcms Roadmap

## Vision

oxcms aims to be the definitive Rust color management system by combining:
- **moxcms**: Rust safety, SIMD performance, modern API
- **lcms2**: Full ICC v4.4, CMYK, DeviceLink, CIECAM02
- **skcms**: OSS-Fuzz hardening, HDR (PQ/HLG), Chrome-hardened
- **qcms**: Firefox battle-tested reliability

## Current State

**Phase 1 Complete** - Test infrastructure is working:
- [x] Project structure with workspace
- [x] Parity test framework (185 tests passing)
- [x] Cross-CMS comparison (moxcms, lcms2, qcms, skcms)
- [x] CI workflow (all platforms, stable + nightly)
- [x] Math difference documentation
- [x] ARM64 NEON bug identified and fixed in moxcms fork

**oxcms-core** is currently a thin wrapper over moxcms. The next phases build our own implementation.

## Phase 2: Independent Implementation

### Goal
Replace moxcms wrapper with our own implementation that:
1. Matches lcms2 output exactly (within floating-point tolerance)
2. Maintains moxcms-level performance
3. Has clear, auditable code

### Step 2.1: Core Color Math
Build foundational color math modules independent of moxcms:

- [ ] `color/xyz.rs` - CIE XYZ color space
- [ ] `color/lab.rs` - CIELAB (L*a*b*) color space
- [ ] `color/rgb.rs` - RGB color space primitives
- [ ] `color/white_point.rs` - D50, D65, DCI white points
- [ ] `math/matrix.rs` - 3x3 matrix operations
- [ ] `math/gamma.rs` - sRGB gamma, parametric curves
- [ ] `math/interpolation.rs` - Linear, tetrahedral interpolation

**Validation**: Each module has tests comparing output to lcms2 reference values.

### Step 2.2: ICC Profile Parsing
Implement ICC profile parsing without moxcms:

- [ ] `icc/header.rs` - 128-byte ICC header
- [ ] `icc/tag_table.rs` - Tag table parsing
- [ ] `icc/tags/mod.rs` - Tag dispatcher
- [ ] `icc/tags/xyz.rs` - XYZ tags (rXYZ, gXYZ, bXYZ)
- [ ] `icc/tags/trc.rs` - TRC curves (rTRC, gTRC, bTRC)
- [ ] `icc/tags/text.rs` - Text/description tags
- [ ] `icc/tags/lut.rs` - LUT tags (A2B, B2A, mAB, mBA)
- [ ] `icc/v2.rs` - ICC v2.x specifics
- [ ] `icc/v4.rs` - ICC v4.x specifics

**Validation**: Parse lcms2 test profiles, compare parsed values.

### Step 2.3: Matrix-Shaper Transforms
Implement the simplest transform type (covers 90% of use cases):

- [ ] `transform/pipeline.rs` - Transform pipeline abstraction
- [ ] `transform/matrix_shaper.rs` - Matrix-shaper profile transforms
- [ ] `transform/trc_eval.rs` - TRC curve evaluation
- [ ] `transform/chromatic_adaptation.rs` - Bradford adaptation

**Validation**: sRGB→sRGB, sRGB→P3, P3→sRGB transforms match lcms2 exactly.

### Step 2.4: LUT-Based Transforms
Implement LUT (Look-Up Table) based transforms:

- [ ] `lut/lut1d.rs` - 1D LUT
- [ ] `lut/lut3d.rs` - 3D LUT (A2B/B2A)
- [ ] `lut/interpolation.rs` - Trilinear, tetrahedral
- [ ] `lut/clut.rs` - Color LUT handling

**Validation**: CMYK profiles, device link profiles match lcms2.

### Step 2.5: SIMD Optimization
Add SIMD acceleration matching moxcms performance:

- [ ] `simd/mod.rs` - SIMD dispatch
- [ ] `simd/sse4.rs` - SSE4.1 paths
- [ ] `simd/avx2.rs` - AVX2 paths
- [ ] `simd/neon.rs` - ARM NEON paths
- [ ] Scalar fallback always available

**Validation**: SIMD output matches scalar exactly. Benchmark against moxcms.

## Phase 3: Feature Parity with lcms2

### Step 3.1: CMYK Support
- [ ] CMYK profile parsing
- [ ] RGB→CMYK transforms
- [ ] CMYK→RGB transforms
- [ ] Black point compensation
- [ ] Ink limiting

### Step 3.2: DeviceLink Profiles
- [ ] DeviceLink profile parsing
- [ ] DeviceLink application
- [ ] DeviceLink creation

### Step 3.3: Advanced Features
- [ ] Named color profiles (Pantone, etc.)
- [ ] Multi-processing element (MPE) profiles
- [ ] CIECAM02 color appearance model
- [ ] Gamut mapping

## Phase 4: Beyond lcms2

### Step 4.1: HDR Support
- [ ] PQ (Perceptual Quantizer) transfer function
- [ ] HLG (Hybrid Log-Gamma) transfer function
- [ ] BT.2100 color spaces
- [ ] HDR→SDR tone mapping

### Step 4.2: Security Hardening
- [ ] OSS-Fuzz integration
- [ ] cargo-fuzz harnesses
- [ ] Malformed profile handling
- [ ] Memory safety audit

### Step 4.3: Profile Creation
- [ ] Profile builder API
- [ ] ICC profile serialization
- [ ] Profile optimization

## Success Criteria

### Accuracy
- [ ] DeltaE2000 < 0.0001 vs lcms2 for all standard transforms
- [ ] Bit-exact for identity transforms
- [ ] All lcms2 testbed tests pass

### Performance
- [ ] At least as fast as moxcms (3x+ faster than lcms2)
- [ ] SIMD acceleration on x86_64 (SSE4, AVX2) and ARM64 (NEON)

### Completeness
- [ ] Full ICC v4.4 support
- [ ] CMYK workflow support
- [ ] DeviceLink profiles
- [ ] All rendering intents

### Safety
- [ ] Zero unsafe code in non-SIMD paths
- [ ] All unsafe SIMD code audited and documented
- [ ] No panics on malformed input
- [ ] Fuzzing coverage of all parse paths

## Non-Goals (For Now)

These can be added later but are not in initial scope:
- GPU compute shaders
- Embedded systems (no_std)
- WebAssembly target
- C FFI bindings
- iccMAX support

## Architecture Decisions

See [ARCHITECTURE.md](../docs/ARCHITECTURE.md) for detailed rationale.

1. **Start from moxcms as reference** - Already Rust, good SIMD
2. **lcms2 as accuracy reference** - Industry standard
3. **Layer-by-layer validation** - Each module tested independently
4. **All differences documented** - No silent divergence
