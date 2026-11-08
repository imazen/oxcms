# Argyll CMYK oracle

`cmyk_argyll_oracle.c` evaluates a CMYK profile through Argyll's icclib,
then converts XYZ PCS through the exact supplied RGB destination profile.
It is test equipment, not an oxcms production backend or new Rust API.

Build with the ICC sources from the official
[ArgyllCMS 3.5.0 source archive](https://www.argyllcms.com/Argyll_V3.5.0_src.zip)
(SHA-256 `f8576ce5589fd15620abb73ff049ea31f55ddbd1bba6d1ffa87452658e7bc85f`):

```sh
cc -O2 -I external/argyllcms/icc tools/cmyk_argyll_oracle.c \
  external/argyllcms/icc/icc.c -lm -o argyll-cmyk-oracle
./argyll-cmyk-oracle SOURCE.icc DEST.icc INPUT.f32 OUTPUT.f32 INTENT
```

Input contains native-endian F32 C/M/Y/K values normalized to [0,1], where
zero means no ink. Output contains native-endian F32 RGB. INTENT is literal
0,1,2,3; no black-point compensation is applied. `-` writes binary output to
stdout. Lookup clipping warnings are counted on stderr, not silently treated
as proof of extended-range support. A parse or lookup failure is an error.

A bounded initial control uses the existing `cmyk_layers.icc` fixture and
Argyll's bundled sRGB profile, checking no ink, K-only black, cyan and full ink
against C Little CMS 2.20 and the frozen Rust port. The common profiles,
normalized ink convention, intent and absence of BPC are essential: comparison
with a BPC-enabled production transform measures a different operation.
