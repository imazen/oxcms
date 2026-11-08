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

## Reproduction

Download the linked official archive to a durable local path. Run
`just argyll-extract ARCHIVE external/argyllcms/icc` in a fresh checkout,
then `just argyll-cmyk-build external/argyllcms/icc FRESH_BINARY`.
Extraction checks SHA-256 and refuses an existing destination. Reference code
and profiles remain untracked; no generated binary is bundled with oxcms.
The normal C build passes. A separate optional `-Werror` experiment fails on
unused static declarations/comments in the external Argyll header; its failure
is retained and does not establish a defect in the oracle's numeric output.

Build zenpixels' standalone `cms-contract` examples `profile_eval` and
`cmyk_layout_contract`. Compile its
`scripts/icc-gen/validation/profile_eval.c` with the frozen LCMS 2.20 reference
sources. `just cmyk-matrix C_LCMS RUST_EVAL ARGYLL FRESH_OUTPUT` runs the
full matrix; append `--quick` for the smaller witness set. The layout and native
recipes require explicit manifests/inputs and fresh output directories. Missing
files, nonfinite output and CMS execution failures are errors. No output RGB
planes are retained: reports contain scalar differences, hashes and witnesses.

## Measured scope, 2026-11-08

The matrix covers 16 unique existing CMYK profiles, exact sRGB and Display P3
target ICCs, 5,721 normalized ink samples, all four literal intents without
added BPC, and separate relative+BPC controls: 640/640 executions succeeded.
Endpoints, K-only and CMY-neutral ramps, axes and deterministic mixed inks are
included. Rust LCMS and its production adapter match C LCMS 2.20 exactly in
all tested corresponding configurations. This is a port differential gate,
not a proof that every CMS must choose the same interpolation/rendering policy.

Argyll 3.5.0/icclib 3.0 and MoxCMS 0.9.2 differ from LCMS. A minimal 836-byte
qcms profile with a 2-point CMYK grid is the largest relative-intent outlier:
maximum normalized RGB differences 0.5675196647644043 (Argyll) and
0.3399251401424408 (raw MoxCMS). Retain it; do not average it away or treat
reader agreement as a standards verdict. Comparisons report raw and clipped
errors, quantized U8/U16 differences, out-of-range counts and exact worst inputs.

On one 512×512 native JXL CMYK input, with a common target ICC and BPC off,
Argyll differs by at most 0.003548193722963333 normalized RGB, mean
0.000003752357372377446; the largest U8 quantized difference is one code.
Rust LCMS matches C exactly. Raw MoxCMS differs by at most
0.05359313450753689 on this control. The production named-sRGB/BPC-on operation
is separate: Rust LCMS is byte-identical to the independent reference, while
MoxCMS exceeds the retained 0.0013 bound in 937/786,432 components, maximum
0.006689250469207764. No tolerance changed.

Strided native row checks cover four CMYK input formats into same-depth
RGB/RGBA, 16 profiles and two targets. LCMS passes 256/256 cases with exact C
native output; MoxCMS passes 192/256 and explicitly declines all 64 CMYK16
cases. Every accepted case preserves opaque alpha, source bytes and padding.
This qualifies source CMYK conversion; float CMYK destinations are not covered.

The Rust probe uses zenpixels' pinned MoxCMS 0.9.2 and Rust LCMS 55b59476,
not oxcms' older vendored MoxCMS. Existing workspace failures listed in the
changelog were not reclassified as passing by these independent tools.
