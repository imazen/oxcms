# Math Differences Between CMS Implementations

This document tracks all mathematical differences between color management systems.

## Methodology

We compare output pixel values for identical input across:
- **moxcms** (pure Rust, our current backend)
- **lcms2** (C, industry standard reference)
- **qcms** (pure Rust, Firefox's CMS)
- **skcms** (C++, Chrome's CMS, via FFI)

Differences are measured using **DeltaE2000**, the industry-standard perceptual color difference metric.

## Current Status (2025-12-27)

### Summary

| Comparison | Transform | Platform | Mean ΔE | Max ΔE | Status |
|------------|-----------|----------|---------|--------|--------|
| moxcms vs lcms2 | sRGB→sRGB | x86_64 | 0.0000 | 0.0000 | IDENTICAL |
| moxcms vs lcms2 | sRGB→sRGB | ARM64 | 0.0000 | 0.0000 | IDENTICAL (after fix) |
| moxcms vs qcms | sRGB→sRGB | all | 0.0000 | 0.0000 | IDENTICAL |
| qcms vs lcms2 | sRGB→sRGB | all | 0.0000 | 0.0000 | IDENTICAL |

**All four CMS implementations produce identical output for sRGB identity transforms.**

### Test Coverage

| Category | Samples | Differences | Max ΔE | Status |
|----------|---------|-------------|--------|--------|
| Grayscale (0-255) | 256 | 0 | 0.0000 | PASS |
| Primary colors | 8 | 0 | 0.0000 | PASS |
| Skin tones | 64 | 0 | 0.0000 | PASS |
| Gamut boundary | 64 | 0 | 0.0000 | PASS |
| Random (seed 42) | 100 | 0 | 0.0000 | PASS |
| Color cube sample | 4913 | 0 | 0.0000 | PASS |

## DeltaE Thresholds

| ΔE Value | Perception | Test Result |
|----------|------------|-------------|
| < 0.1 | Invisible | PASS |
| 0.1 - 0.5 | Barely visible | PASS |
| 0.5 - 1.0 | Threshold of perception | PASS |
| 1.0 - 2.0 | Visible to trained observers | **FAIL** |
| > 2.0 | Obvious | **FAIL** |

## Known Issues (Fixed)

### ARM64 NEON Bug in moxcms (FIXED)

**Status**: Fixed in oxcms fork (PR #1)

**Bug**: Copy-paste error in NEON fixed-point code paths caused the blue channel of every second pixel to use the wrong source register.

**Location**:
- `external/moxcms/src/conversions/neon/rgb_xyz_q2_13_opt.rs`
- `external/moxcms/src/conversions/neon/rgb_xyz_q1_30_opt.rs`

**Root Cause**:
```rust
// BEFORE (BUG): Used vr0 for second pixel's blue channel
dst0[dst_cn.b_i() + dst_channels] =
    self.profile.gamma[vget_lane_u16::<2>(vr0) as usize];  // WRONG!

// AFTER (FIXED): Use vr1 for second pixel
dst0[dst_cn.b_i() + dst_channels] =
    self.profile.gamma[vget_lane_u16::<2>(vr1) as usize];  // Correct
```

**Impact**: DeltaE errors up to 40+ on ARM64 macOS before fix.

**Lesson**: Multi-pixel SIMD processing is error-prone. Always test SIMD against scalar reference.

## CMS Implementation Notes

### lcms2

- Industry standard, most complete ICC support
- Uses floating-point math throughout
- Reference for all other implementations

### moxcms

- Pure Rust with SIMD (SSE, AVX2, NEON)
- Fixed-point math in some NEON paths for performance
- Some paths process multiple pixels at once (source of ARM64 bug)

### qcms

- Firefox's CMS, pure Rust
- More conservative feature set
- No grayscale transform support (panics on Gray8)
- In-place transform API only

### skcms

- Chrome's CMS, C++ with excellent security
- Fuzzing-hardened profile parsing
- HDR support (PQ, HLG)

## Transform-Specific Notes

### sRGB Identity Transform

All CMS produce identical output. This is the baseline for all other comparisons.

### sRGB → Display P3

Expected perceptual changes (not errors):
- sRGB red [255,0,0] → P3 [~234,~51,~35] - less saturated in P3
- Pure primaries shift most (as expected for gamut mapping)
- Neutrals (black, white, grays) unchanged

### Rendering Intents

| Intent | lcms2 vs moxcms | lcms2 vs qcms |
|--------|-----------------|---------------|
| Perceptual | IDENTICAL | IDENTICAL |
| Relative Colorimetric | IDENTICAL | IDENTICAL |
| Saturation | IDENTICAL | IDENTICAL |
| Absolute Colorimetric | IDENTICAL | IDENTICAL |

All four rendering intents produce identical results across implementations.

## Test Commands

```bash
# Run parity tests with output
cargo test -p cms-tests lcms2_parity -- --nocapture

# Run math difference documentation
cargo test -p cms-tests math_differences -- --nocapture

# Generate full report
cargo test -p cms-tests generate_difference_report -- --nocapture

# Run ARM64 diagnostics
cargo test -p cms-tests diagnose_arm64 -- --nocapture
```

## Adding New Differences

When a difference is found:

1. **Document** the specific input values that differ
2. **Record** output from each implementation
3. **Calculate** DeltaE2000
4. **Investigate** root cause
5. **Either**:
   - Fix the implementation to match reference
   - Document the difference with justification

### Template

```markdown
### [Difference Name]

**Status**: [Open | Fixed | Documented]

**Affected**: [Platforms/transforms affected]

**Observed ΔE**: [Mean and max]

**Root Cause**: [Technical explanation]

**Resolution**: [Fix applied or justification for accepting difference]
```

## Future Testing

Additional comparisons to implement:

- [ ] Profile-to-profile transforms (P3, AdobeRGB, etc.)
- [ ] CMYK transforms
- [ ] Lab/XYZ conversions
- [ ] Different rendering intents with non-identity transforms
- [ ] 16-bit precision
- [ ] Floating-point transforms
- [ ] LUT-based profiles
- [ ] DeviceLink profiles

## References

- [DeltaE2000 Formula](http://www.brucelindbloom.com/index.html?Eqn_DeltaE_CIE2000.html)
- [ICC Specification v4.4](https://www.color.org/specification/ICC.1-2022-05.pdf)
- [lcms2 Documentation](https://www.littlecms.com/lcms-2.16/lcms2.pdf)
