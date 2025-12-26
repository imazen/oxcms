# Test Status Tracking

Last updated: 2025-12-26

## Summary

| Source | Total Tests | Passing | Failing | Skipped | Coverage |
|--------|-------------|---------|---------|---------|----------|
| oxcms-core | 6 | 6 | 0 | 0 | Core API |
| cms-tests lib | 6 | 6 | 0 | 0 | Accuracy |
| Corpus validation | 3 | 3 | 0 | 0 | Parsing |
| Color space tests | 8 | 8 | 0 | 0 | Lab/XYZ/Gray |
| Extended parity | 7 | 7 | 0 | 0 | Transforms |
| lcms2 parity | 3 | 3 | 0 | 0 | Parity |
| Math differences | 4 | 4 | 0 | 0 | Documentation |
| moxcms parity | 2 | 2 | 0 | 0 | Consistency |
| Rendering intents | 7 | 7 | 0 | 0 | Intent comparison |
| qcms parity | 11 | 11 | 0 | 0 | Firefox CMS |
| Corpus parity | 4 | 4 | 0 | 0 | 110 ICC profiles |
| Doc tests | 1 | 1 | 0 | 0 | Examples |
| **Total** | **62** | **62** | **0** | **0** | **100%** |

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

| Test | moxcms | lcms2 | Status |
|------|--------|-------|--------|
| sRGB → CMYK | ? | ? | TBD |
| CMYK → sRGB | ? | ? | TBD |
| CMYK → CMYK | ? | ? | TBD |

### Lab/XYZ Transforms

| Test | moxcms | lcms2 | Status |
|------|--------|-------|--------|
| RGB → Lab | ? | ? | TBD |
| Lab → RGB | ? | ? | TBD |
| RGB → XYZ | ? | ? | TBD |
| XYZ → RGB | ? | ? | TBD |

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
