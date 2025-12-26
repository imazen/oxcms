# Test Status Tracking

Last updated: 2025-12-25

## Summary

| Source | Total Tests | Passing | Failing | Skipped | Coverage |
|--------|-------------|---------|---------|---------|----------|
| moxcms | TBD | - | - | - | - |
| lcms2 testbed | TBD | - | - | - | - |
| skcms profiles | TBD | - | - | - | - |
| qcms | TBD | - | - | - | - |
| **Total** | **TBD** | **-** | **-** | **-** | **-** |

## Test Categories

### Profile Parsing

| Test | moxcms | lcms2 | skcms | qcms | Status |
|------|--------|-------|-------|------|--------|
| Parse sRGB v2 | ? | ? | ? | ? | TBD |
| Parse sRGB v4 | ? | ? | ? | ? | TBD |
| Parse Display P3 | ? | ? | ? | ? | TBD |
| Parse Adobe RGB | ? | ? | ? | ? | TBD |
| Parse CMYK | ? | ? | ? | ? | TBD |
| Parse DeviceLink | ? | ? | ? | ? | TBD |
| Parse NamedColor | ? | ? | ? | ? | TBD |
| Malformed profile | ? | ? | ? | ? | TBD |

### RGB Transforms

| Test | moxcms | lcms2 | skcms | qcms | DeltaE |
|------|--------|-------|-------|------|--------|
| sRGB → sRGB | ? | ? | ? | ? | TBD |
| sRGB → P3 | ? | ? | ? | ? | TBD |
| P3 → sRGB | ? | ? | ? | ? | TBD |
| sRGB → AdobeRGB | ? | ? | ? | ? | TBD |
| Wide gamut roundtrip | ? | ? | ? | ? | TBD |

### CMYK Transforms

| Test | moxcms | lcms2 | skcms | qcms | DeltaE |
|------|--------|-------|-------|------|--------|
| sRGB → CMYK | ? | ? | N/A | ? | TBD |
| CMYK → sRGB | ? | ? | N/A | ? | TBD |
| CMYK → CMYK | ? | ? | N/A | ? | TBD |

### Lab/XYZ Transforms

| Test | moxcms | lcms2 | skcms | qcms | DeltaE |
|------|--------|-------|-------|------|--------|
| RGB → Lab | ? | ? | ? | ? | TBD |
| Lab → RGB | ? | ? | ? | ? | TBD |
| RGB → XYZ | ? | ? | ? | ? | TBD |
| XYZ → RGB | ? | ? | ? | ? | TBD |

### Rendering Intents

| Test | moxcms | lcms2 | skcms | qcms | Status |
|------|--------|-------|-------|------|--------|
| Perceptual | ? | ? | ? | ? | TBD |
| Relative Colorimetric | ? | ? | ? | ? | TBD |
| Saturation | ? | ? | ? | ? | TBD |
| Absolute Colorimetric | ? | ? | ? | ? | TBD |

### Edge Cases

| Test | moxcms | lcms2 | skcms | qcms | Status |
|------|--------|-------|-------|------|--------|
| Empty profile | ? | ? | ? | ? | TBD |
| Truncated profile | ? | ? | ? | ? | TBD |
| Invalid tag count | ? | ? | ? | ? | TBD |
| Invalid tag offset | ? | ? | ? | ? | TBD |
| Recursive tags | ? | ? | ? | ? | TBD |

## Update Process

1. Run `cargo test -p cms-tests`
2. Update this file with results
3. Commit with message: `docs: update test status YYYY-MM-DD`

## Legend

- ✅ Passing
- ❌ Failing
- ⚠️ Partial
- ⏭️ Skipped
- ? Not yet tested
- N/A Not applicable
