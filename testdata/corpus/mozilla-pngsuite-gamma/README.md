# Mozilla PngSuite Gamma Tests

Source: https://hg.mozilla.org/mozilla-central/file/tip/image/test/reftest/pngsuite-gamma/
License: MPL-2.0 (Mozilla), PngSuite images from http://www.schaik.com/pngsuite/

## Test Structure

Each PNG file has a matching HTML reference file containing a 32x32 pixel table
with the expected gamma-corrected color values.

## File Naming Convention

`gXXnYYY.png` where:
- `XX` = gamma value (03=0.35, 04=0.45, 05=0.55, 07=0.70, 10=1.00, 25=2.50)
- `n` = non-interlaced
- `YYY` = color type:
  - `0g16` = grayscale, 16-bit
  - `2c08` = truecolor RGB, 8-bit
  - `3p04` = paletted, 4-bit

## Gamma Values Tested

| Gamma | Files | Purpose |
|-------|-------|---------|
| 0.35 | g03n* | Very dark encoding, aggressive expansion |
| 0.45 | g04n* | CRT gamma (1/2.2) |
| 0.55 | g05n* | Moderate gamma |
| 0.70 | g07n* | Higher gamma |
| 1.00 | g10n* | Linear (no correction) - baseline |
| 2.50 | g25n* | Very bright encoding, compression |

## Validation

The reftest framework renders both the PNG and its HTML reference,
then compares them pixel-by-pixel. If gamma correction is applied correctly,
they should match exactly.

## Usage

```
== g03n0g16.png g03n0g16.html
```

The `==` means the two renderings must be pixel-identical.
