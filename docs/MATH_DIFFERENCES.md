# Math Differences Between CMS Implementations

This document tracks all mathematical differences between color management systems.

## Methodology

We compare output pixel values for identical input across:
- **moxcms** (pure Rust, our baseline)
- **lcms2** (C, industry standard)
- **skcms** (via saved reference outputs)
- **qcms** (pure Rust, Firefox)

Differences are measured using **deltaE2000**, the industry-standard perceptual color difference metric.

## Current Status

As of 2025-12-25, initial testing shows:

| Comparison | Transform | Mean ΔE | Max ΔE | Status |
|------------|-----------|---------|--------|--------|
| moxcms vs lcms2 | sRGB→sRGB | 0.0000 | 0.0000 | IDENTICAL |

## Difference Thresholds

| ΔE Value | Perception |
|----------|------------|
| < 0.1 | Invisible |
| 0.1 - 0.5 | Barely visible |
| 0.5 - 1.0 | Threshold of perception |
| 1.0 - 2.0 | Visible to trained observers |
| > 2.0 | Obvious |

## Known Differences

### None Currently Documented

All tested sRGB transforms produce identical results between moxcms and lcms2.

## Test Commands

```bash
# Run parity tests with output
cargo test -p cms-tests lcms2_parity -- --nocapture

# Run math difference documentation
cargo test -p cms-tests math_differences -- --nocapture

# Generate full report
cargo test -p cms-tests generate_difference_report -- --nocapture
```

## Adding New Differences

When a difference is found:

1. Document the specific input values that differ
2. Record output from each implementation
3. Calculate deltaE2000
4. Investigate root cause
5. Either:
   - Fix the implementation to match reference
   - Document the difference with justification

## Future Testing

Tests to add:
- [ ] Profile-to-profile transforms (P3, AdobeRGB, etc.)
- [ ] CMYK transforms
- [ ] Lab/XYZ conversions
- [ ] Different rendering intents
- [ ] 16-bit precision
- [ ] Floating-point transforms
