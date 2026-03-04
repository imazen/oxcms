# moxcms CMYK Layout Notes

Findings from integrating moxcms 0.8.0 into imageflow's CMS backend.

## CMYK Layout: Use `Layout::Rgba`, NOT `Layout::Cmyka`

The `Layout` enum has a `Cmyka` variant, but it's for **5-channel** CMYK+Alpha data (5 bytes/pixel). Standard 4-channel CMYK data (as output by mozjpeg for CMYK JPEGs) must use `Layout::Rgba` instead.

moxcms enforces this via `DataColorSpace::check_layout()` in `profile.rs:213`:

```rust
DataColorSpace::Cmyk => layout != Layout::Rgba,
```

If you pass `Layout::Cmyka` with a CMYK ICC profile, you get `CmsError::InvalidLayout`.

### Correct usage

```rust
let src = ColorProfile::new_from_slice(cmyk_icc_bytes)?;
let dst = ColorProfile::new_srgb();

// CMYK data is 4 bytes/pixel — use Layout::Rgba, NOT Layout::Cmyka
let transform = src.create_transform_8bit(
    Layout::Rgba,   // source: 4-channel CMYK
    &dst,
    Layout::Rgba,   // dest: 4-channel RGBA
    TransformOptions::default(),
)?;

// Input: [C, M, Y, K, C, M, Y, K, ...]
// Output: [R, G, B, A, R, G, B, A, ...]
transform.transform(&cmyk_input, &mut rgba_output)?;
```

### Channel counts

| Layout | Channels | Use for |
|--------|----------|---------|
| `Rgba` | 4 | RGB+Alpha OR CMYK (semantics from ICC profile) |
| `Cmyka` | 5 | CMYK + Alpha (5 bytes/pixel) |
| `Rgb` | 3 | RGB (no alpha) |

### CMYK → RGB mismatch detection

When an ICC profile has `DataColorSpace::Cmyk` but the image data is actually RGB (e.g., an RGB JPEG with a CMYK ICC profile embedded by mistake), moxcms will happily create the transform and produce garbage output. Unlike lcms2, which rejects the channel count mismatch (lcms2's BGRA_8 format encodes 3+1 channels, not 4 uniform channels), moxcms sees `Layout::Rgba` = 4 channels = matches CMYK's 4 channels.

You must validate this yourself:

```rust
let src = ColorProfile::new_from_slice(icc_bytes)?;
if src.color_space == DataColorSpace::Cmyk {
    return Err("ICC profile is CMYK but image data is RGB");
}
```

## Integration results: moxcms vs lcms2

Tested with imageflow's visual regression suite. Both CMS backends produce nearly identical output:

- **CMYK JPEG** (USWebCoatedSWOP profile): max channel delta 3, ~8.5% pixels differ
- **ICC v4 profiles**: small delta differences
- **gAMA+cHRM (PNG)**: max channel delta 1-2
- **ICC RGB profiles**: max channel delta 1-3

All differences are within expected CMS implementation tolerances (different interpolation, different intermediate precision).

## mozjpeg CMYK byte inversion

mozjpeg outputs CMYK JPEGs with inverted byte values (255-C, 255-M, 255-Y, 255-K). This matches the Adobe convention. You must un-invert before passing to the ICC transform:

```rust
for byte in cmyk_row.iter_mut() {
    *byte = 255 - *byte;
}
```

lcms2 handles this automatically with `PixelFormat::CMYK_8_REV`. moxcms has no REV layout — manual un-inversion required.
