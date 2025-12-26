# Test Status Tracking

Last updated: 2025-12-25

## Summary

| Source | Total Tests | Passing | Failing | Skipped | Coverage |
|--------|-------------|---------|---------|---------|----------|
| oxcms-core | 6 | 6 | 0 | 0 | Core API |
| cms-tests lib | 6 | 6 | 0 | 0 | Accuracy |
| Corpus validation | 3 | 3 | 0 | 0 | Parsing |
| Extended parity | 7 | 7 | 0 | 0 | Transforms |
| lcms2 parity | 3 | 3 | 0 | 0 | Parity |
| Math differences | 4 | 4 | 0 | 0 | Documentation |
| moxcms parity | 2 | 2 | 0 | 0 | Consistency |
| Doc tests | 1 | 1 | 0 | 0 | Examples |
| **Total** | **32** | **32** | **0** | **0** | **100%** |

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

| Test | moxcms | lcms2 | Status |
|------|--------|-------|--------|
| Perceptual | ✅ | ✅ | PASS |
| Relative Colorimetric | ? | ? | TBD |
| Saturation | ? | ? | TBD |
| Absolute Colorimetric | ? | ? | TBD |

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
