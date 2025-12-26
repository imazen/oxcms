# Implementation Log

## 2025-12-25: Project Initialization

### Created
- Workspace structure with `oxcms-core` and `cms-tests` crates
- Documentation in `docs/`, `plans/`, `tracking/`
- CI workflow for GitHub Actions
- Test profiles fetching script
- Parity test framework comparing moxcms, lcms2

### Initial Test Results
- All 18 tests passing
- moxcms and lcms2 produce identical output for sRGB identity transforms
- Test infrastructure ready for expansion

---

## 2025-12-25: oxcms-core Implementation (Phase 1)

### Changed
- Rewrote `oxcms-core` to wrap moxcms with a stable API
- Added comprehensive `ColorProfile` wrapper with:
  - Built-in profiles: sRGB, Display P3, Adobe RGB, BT.2020, BT.709
  - Profile parsing from ICC bytes
  - Profile metadata access
- Added `Transform` wrapper with 8/16/32-bit support
- Added `Layout` and `RenderingIntent` enums
- Added comprehensive error types with `#[non_exhaustive]`

### Added
- `extended_parity.rs` test suite with 7 new tests:
  - sRGB to P3 parity
  - Transform determinism
  - Round-trip accuracy
  - Extreme color values
  - lcms2 sRGB identity comparison
  - Loaded profile parity
  - 16-bit precision tests

### Test Results
- 32 tests passing (up from 18)
- moxcms and lcms2 produce identical sRGB identity output
- sRGB→P3 shows expected ΔE 2.7-4.3 for saturated colors
- Round-trip accuracy excellent for neutral colors

### Key Findings
1. **lcms2 vs moxcms parity**: Identical for sRGB identity (max diff: 0)
2. **Color gamut mapping**: sRGB red → P3 becomes [234, 51, 35] (ΔE: 4.14)
3. **Precision**: 16-bit and 8-bit transforms produce consistent results

### Next Steps
1. Add CMYK transform tests with external profiles
2. Test with external ICC profiles (fetch script)
3. Add fuzzing harnesses

---

## 2025-12-25: Extended Test Coverage

### Added
- `color_space_tests.rs` with 8 tests:
  - CMYK profile loading support
  - Lab conversion accuracy (< 0.02 ΔE)
  - XYZ round-trip testing
  - Grayscale ↔ RGB transforms
  - Bit depth consistency (8-bit vs 16-bit)
  - D50 white point verification
  - Alpha channel preservation

- `rendering_intents.rs` with 7 tests:
  - All 4 rendering intents tested
  - lcms2 comparison for all intents
  - Identity transform verification

### Test Results
- 47 tests passing (up from 32)
- All 4 rendering intents produce identical results between moxcms and lcms2
- Lab conversion accurate to within 0.02 ΔE
- Grayscale luminance correctly weights green > red > blue

### Key Findings
1. **Rendering intent parity**: All 4 intents match lcms2 exactly
2. **Lab accuracy**: Computed Lab matches reference values within 0.02 ΔE
3. **White point**: All profiles correctly use D50 PCS (ICC standard)
4. **Luminance**: Green (219) > Red (129) > Blue (71) for pure primaries

---

## 2025-12-26: qcms Parity Testing

### Added
- `qcms_parity.rs` test suite with 11 tests:
  - Profile creation (sRGB, Gray, XYZ D50)
  - sRGB identity transform
  - qcms vs lcms2 comparison
  - qcms vs moxcms comparison
  - Three-way CMS comparison
  - All 4 rendering intents
  - RGBA transform with alpha preservation
  - Grayscale profile (creation only, transforms not supported)
  - ICC profile parsing
  - Determinism verification
  - All intents vs lcms2 comparison

### Test Results
- 58 tests passing (up from 47)
- All three CMS (qcms, moxcms, lcms2) produce **identical** sRGB identity output
- All 4 rendering intents match exactly between qcms and lcms2
- Maximum channel difference: **0**

### Key Findings
1. **Three-way parity**: qcms, moxcms, and lcms2 all produce identical output for sRGB identity transforms
2. **Intent parity**: All 4 intents match between qcms and lcms2 (diff: 0)
3. **Determinism**: qcms produces identical results across multiple iterations
4. **Alpha preservation**: RGBA transforms correctly preserve alpha channel

### qcms Limitations Discovered
1. **No grayscale transforms**: qcms panics on Gray8 DataType (expects input/output types to match profile types)
2. **In-place API only**: Unlike moxcms/lcms2, qcms only supports in-place transforms
3. **Limited introspection**: Profile fields like `is_srgb` are private
4. **ICC parsing quirks**: Some ICC profiles fail to parse that work with lcms2

---

## 2025-12-26: ICC Profile Corpus

### Added
- **110 ICC profiles** from multiple sources:
  - lcms2 testbed (11 profiles) - MIT license
  - qcms/Mozilla (9 profiles + 6 fuzz samples) - MIT license
  - skcms/Google (~70 profiles) - BSD-3 license
  - ICC.org (3 reference profiles) - freely redistributable
  - Compact-ICC (9 minimal profiles) - CC0 public domain

- **14 test images** with embedded profiles:
  - Skia test images (3 files)
  - Pillow test images (2 files)
  - Compact-ICC minimal profiles (9 files)

- `corpus_parity.rs` test suite with 4 tests:
  - Profile parsing across all CMS
  - Transform parity testing
  - sRGB consistency validation
  - Profile category analysis

### Test Results
- 62 tests passing (up from 58)
- Total corpus size: 4.8MB

### Corpus Parsing Results
| CMS | Profiles Parsed | Percentage |
|-----|-----------------|------------|
| lcms2 | 108/119 | 91% |
| qcms | 87/119 | 73% |
| moxcms | 76/119 | 64% |
| All 3 | 76/119 | 64% |

### Transform Parity Results (55 standard RGB profiles)
- Identical output: 24 profiles (44%)
- Small differences (<=2): 19 profiles (35%)
- Large differences (>2): 12 profiles (22%)
- Overall parity: **78% acceptable**

### Key Findings
1. **lcms2 is most compatible** - parses 91% of profiles
2. **moxcms is strictest** - only 64% parsed (stricter validation)
3. **Transform parity varies** - device-specific and v4 profiles show most differences
4. **sRGB consistency excellent** - all sRGB variants produce identical output

---

## Template for Future Entries

```markdown
## YYYY-MM-DD: Description

### Changed
- List of changes

### Fixed
- List of fixes

### Added
- List of additions

### Test Status
- X tests passing, Y failing

### Notes
- Additional context
```
