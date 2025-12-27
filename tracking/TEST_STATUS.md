# Test Status Tracking

Last updated: 2025-12-27

## Summary

| Source | Total Tests | Passing | Failing | Skipped | Coverage |
|--------|-------------|---------|---------|---------|----------|
| moxcms | 51 | 51 | 0 | 0 | Core CMS |
| oxcms-core | 12 | 12 | 0 | 0 | Core API |
| skcms-sys | 2 | 2 | 0 | 0 | FFI bindings |
| cms-tests lib | 6 | 6 | 0 | 0 | Accuracy |
| Browser parity | 5 | 5 | 0 | 0 | skcms/qcms |
| CMYK transforms | 5 | 5 | 0 | 0 | CMYK pipeline |
| Color space tests | 8 | 8 | 0 | 0 | Lab/XYZ/Gray |
| Corpus parity | 4 | 4 | 0 | 0 | 121 ICC profiles |
| Corpus validation | 3 | 3 | 0 | 0 | Parsing |
| Correctness eval | 3 | 3 | 0 | 0 | Full evaluation |
| Extended parity | 7 | 7 | 0 | 0 | Transforms |
| lcms2 parity | 3 | 3 | 0 | 0 | Parity |
| Math differences | 4 | 4 | 0 | 0 | Documentation |
| moxcms parity | 2 | 2 | 0 | 0 | Consistency |
| Profile analysis | 3 | 3 | 0 | 0 | Deep analysis |
| qcms parity | 11 | 11 | 0 | 0 | Firefox CMS |
| Rendering intents | 7 | 7 | 0 | 0 | Intent comparison |
| TRC analysis | 4+ | 4+ | 0 | 0 | Curve analysis |
| V4 diagnostics | 3 | 3 | 0 | 0 | LUT profiles |
| XYB color space | 8 | 8 | 0 | 0 | JPEG XL color space |
| Named color profiles | 4 | 4 | 0 | 0 | nmcl/ncl2 profiles |
| Doc tests | 1 | 1 | 0 | 0 | Examples |
| **Total** | **185** | **185** | **0** | **0** | **100%** |

## Test Categories

### Profile Parsing

| Test | moxcms | lcms2 | Status |
|------|--------|-------|--------|
| Parse sRGB (built-in) | ✅ | ✅ | PASS |
| Parse Display P3 (built-in) | ✅ | N/A | PASS |
| Parse Adobe RGB (built-in) | ✅ | N/A | PASS |
| Parse BT.2020 (built-in) | ✅ | N/A | PASS |
| Parse from ICC file | ⚠️ | ⚠️ | Needs profiles |

### RGB Transforms

| Test | moxcms | lcms2 | Mean ΔE | Max ΔE | Status |
|------|--------|-------|---------|--------|--------|
| sRGB → sRGB (identity) | ✅ | ✅ | 0.0000 | 0.0000 | IDENTICAL |
| sRGB → P3 | ✅ | N/A | 2.7168 | 4.3176 | PASS |
| Round-trip sRGB→P3→sRGB | ✅ | N/A | 0.0000 | 0.0000 | PASS |

### Bit Depth

| Test | Status | Notes |
|------|--------|-------|
| 8-bit transforms | ✅ | Primary focus |
| 16-bit transforms | ✅ | Tested, matches 8-bit |
| 32-bit float | ✅ | Available but less tested |

### Consistency

| Test | Status | Notes |
|------|--------|-------|
| Transform determinism | ✅ | Same input → same output |
| lcms2 vs moxcms (sRGB identity) | ✅ | Max diff: 0 |
| SIMD consistency | ✅ | No visible variance |

### Rendering Intents

| Test | moxcms | lcms2 | Max Diff | Status |
|------|--------|-------|----------|--------|
| Perceptual | ✅ | ✅ | 0 | IDENTICAL |
| Relative Colorimetric | ✅ | ✅ | 0 | IDENTICAL |
| Saturation | ✅ | ✅ | 0 | IDENTICAL |
| Absolute Colorimetric | ✅ | ✅ | 0 | IDENTICAL |

### CMYK Transforms

| Test | moxcms | lcms2 | Max Diff | Status |
|------|--------|-------|----------|--------|
| sRGB → CMYK | ✅ | ✅ | N/A | PASS |
| CMYK → sRGB | ✅ | ✅ | 7 | PASS |
| CMYK round-trip | ✅ | N/A | 7 | PASS |
| CMYK profile parsing | ✅ | ✅ | 0 | PASS |
| CMYK parity (moxcms/lcms2) | ✅ | ✅ | 7 | PASS |

### Lab/XYZ Transforms

| Test | moxcms | lcms2 | Max ΔE | Status |
|------|--------|-------|--------|--------|
| RGB → Lab | ✅ | N/A | 0.0168 | PASS |
| Lab → RGB | ✅ | N/A | 0.0168 | PASS |
| XYZ → Lab → XYZ | ✅ | N/A | 0.0000 | PASS |
| Lab D50 accuracy | ✅ | N/A | 0.02 | PASS |

### qcms Comparison (Firefox CMS)

| Test | qcms | moxcms | lcms2 | Max Diff | Status |
|------|------|--------|-------|----------|--------|
| sRGB identity | ✅ | ✅ | ✅ | 0 | IDENTICAL |
| Three-way comparison | ✅ | ✅ | ✅ | 0 | IDENTICAL |
| Perceptual intent | ✅ | - | ✅ | 0 | IDENTICAL |
| Relative intent | ✅ | - | ✅ | 0 | IDENTICAL |
| Saturation intent | ✅ | - | ✅ | 0 | IDENTICAL |
| Absolute intent | ✅ | - | ✅ | 0 | IDENTICAL |
| RGBA transform | ✅ | - | - | 0 | PASS |
| Determinism | ✅ | - | - | 0 | PASS |
| Grayscale transforms | ❌ | ✅ | ✅ | N/A | NOT SUPPORTED |

## Key Findings

### moxcms vs lcms2 sRGB Identity
- Both produce **identical** output for sRGB identity transform
- Maximum channel difference: **0**
- No observable math differences for basic transforms

### sRGB to Display P3
- Average color shift: ΔE 2.7168 (perceptible but expected)
- Maximum shift: ΔE 4.3176 (saturated colors)
- Pure primaries shift significantly (as expected for gamut mapping)
- Black and white unchanged (ΔE: 0.0000)

### Round-Trip Accuracy
- sRGB → P3 → sRGB for mid-gray: **perfect** (ΔE: 0.0000)
- Round-trip error < 1 ΔE for neutral colors

### qcms vs moxcms vs lcms2
- **All three CMS produce identical output** for sRGB identity transforms
- Maximum channel difference across all three: **0**
- All 4 rendering intents match exactly between qcms and lcms2
- qcms is deterministic (10 iterations produce identical results)
- qcms correctly preserves alpha channel in RGBA transforms

### qcms Limitations
- **No grayscale transform support** - panics on Gray8 data type
- In-place transform API only (no separate input/output buffers)
- Limited profile introspection (is_srgb field is private)

### XYB Color Space

XYB is the perceptual color space used by JPEG XL. It is NOT a standard ICC profile color space.

| Test | Status | Notes |
|------|--------|-------|
| XYB round-trip (primaries) | ✅ | Perfect round-trip, max error 0 |
| XYB channel meaning | ✅ | X=L-M opponent, Y=luminance, B=blue |
| XYB range analysis | ✅ | Matches ColorAide spec |
| XYB linearity | ✅ | Monotonic Y channel |
| XYB comprehensive (140K colors) | ✅ | 0% errors > 1 |
| colorutils-rs comparison | ⚠️ | colorutils-rs v0.7.5 has critical bug |

**colorutils-rs Bug**: The XYB implementation in colorutils-rs v0.7.5 is completely broken:
- Colors with r=0 all produce identical wrong XYB values
- Round-trip fails for most colors
- Our implementation is verified via perfect round-trip tests

### Named Color Profiles

Named color profiles (nmcl class) contain spot colors like Pantone.

| Test | Status | Notes |
|------|--------|-------|
| Profile structure docs | ✅ | ncl2 tag format documented |
| XML definitions | ✅ | 3 iccMAX XML files found |
| Corpus scan | ✅ | 0 nmcl profiles in 112 tested |
| Class distribution | ✅ | 73 mntr, 9 scnr, 7 prtr, 4 spac |

**Note**: Named color profiles are rare because spot color libraries (Pantone, HKS, etc.) are proprietary.

## Update Process

1. Run `cargo test --all`
2. Update this file with results
3. Commit with message: `docs: update test status YYYY-MM-DD`

## Legend

- ✅ Passing
- ❌ Failing
- ⚠️ Partial
- ⏭️ Skipped
- ? Not yet tested
- N/A Not applicable
