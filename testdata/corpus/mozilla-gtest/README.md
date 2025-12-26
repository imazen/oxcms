# Mozilla gtest Color Test Images

Source: https://hg.mozilla.org/mozilla-central/file/tip/image/test/gtest/
License: MPL-2.0

## Expected Color Values

Test validation from `image/test/gtest/Common.cpp`:

| Image | Dimensions | Expected BGRA | Description |
|-------|-----------|---------------|-------------|
| `green.icc_srgb.webp` | 100x100 | `(0x00, 0xFF, 0x00, 0xFF)` | Pure green, WebP with sRGB ICC |
| `valid-avif-colr-nclx-and-prof.avif` | 1x1 | `(0x00, 0x00, 0x00, 0xFF)` | Black, AVIF with both NCLX and ICC |
| `gray-235-8bit-*-range-*.avif` | 100x100 | `(0xEB, 0xEB, 0xEB, 0xFF)` | Gray 235 |
| `gray-235-10bit-limited-*.avif` | 100x100 | `(0xEA, 0xEA, 0xEA, 0xFF)` | Gray 234 (precision) |
| `gray-235-12bit-limited-*.avif` | 100x100 | `(0xEA, 0xEA, 0xEA, 0xFF)` | Gray 234 (precision) |
| `perf_cmyk.jpg` | 1000x1000 | N/A | CMYK performance test |
| `perf_srgb.png/gif` | varies | N/A | sRGB performance test |
| `perf_gray.jpg/png` | varies | N/A | Grayscale performance test |
| `perf_ycbcr.jpg` | varies | N/A | YCbCr performance test |

## Color Space Variants (gray-235-*.avif)

- **Bit depths**: 8-bit, 10-bit, 12-bit
- **Ranges**: limited-range, full-range
- **Color matrices**: BT.601, BT.709, BT.2020, grayscale

## Validation

Firefox uses pixel-perfect comparison with optional fuzz tolerance of +/-1 per channel.
