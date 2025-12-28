# ICC Profile Corpus - Licenses

This directory contains ICC profiles from multiple open-source projects for parity testing.

## lcms2/ (Little CMS)

**Source:** https://github.com/mm2/Little-CMS/tree/master/testbed
**License:** MIT
**Copyright:** (c) Marti Maria Saguer

Profiles: `bad.icc`, `bad_mpe.icc`, `crayons.icc`, `ibm-t61.icc`, `new.icc`, `test1.icc`, `test3.icc`, `test4.icc`, `test5.icc`, `toosmall.icc`

## qcms/ (Mozilla Firefox CMS)

**Source:** https://hg.mozilla.org/mozilla-central/file/tip/gfx/qcms
**License:** MIT
**Copyright:** (c) 2009-2024 Mozilla Corporation, (c) 1998-2007 Marti Maria

Profiles: `ITU-709.icc`, `ITU-2020.icc`, `sRGB_lcms.icc`, `B2A0-ident.icc`, `displaycal-lut-stripped.icc`, `lcms_samsung_syncmaster.icc`, `lcms_thinkpad_w540.icc`, `parametric-thresh.icc`, `ps_cmyk_min.icc`

Fuzz samples in `qcms/fuzz/` for edge-case testing.

## skcms/ (Google Skia CMS)

**Source:** https://skia.googlesource.com/skcms/
**License:** BSD 3-Clause
**Copyright:** (c) 2018 Google Inc.

### skcms/color.org/
Standard ICC profiles from color.org (freely redistributable).

### skcms/misc/
Real-world profiles from various devices and applications:
- Monitor profiles: `BenQ_GL2450.icc`, `BenQ_RL2455.icc`, `ThinkpadX1YogaV2.icc`, `XPS13_9360.icc`
- Color spaces: `AdobeRGB.icc`, `HD_709.icc`, `Apple_Wide_Color.icc`
- CMYK: `Coated_FOGRA39_CMYK.icc`
- Grayscale: `Dot_Gain_20_Grayscale.icc`, `Gray_Gamma_22.icc`

### skcms/mobile/
Mobile device profiles: `Display_P3_LUT.icc`, `Display_P3_parametric.icc`, `iPhone7p.icc`, `sRGB_LUT.icc`, `sRGB_parametric.icc`

### skcms/fuzz/
Edge-case profiles for fuzzing: truncated, malformed, boundary conditions.

## icc.org/ (ICC Reference Profiles)

**Source:** https://www.color.org/srgbprofiles.xalter
**License:** Freely redistributable with attribution

> "This profile is made available by the International Color Consortium, and may be copied, distributed, embedded, made, used, and sold without restriction."

Profiles: `sRGB2014.icc`, `sRGB_v4_ICC_preference.icc`, `sRGB_ICC_v4_Appearance.icc`

## png-icc-tests/ (SVG Working Group)

**Source:** https://github.com/svgeesus/PNG-ICC-tests
**License:** MIT (implied by public test suite)
**Copyright:** (c) Chris Lilley

Profiles for testing browser ICC profile support in PNG images:
- `sRGB-v2-2014.icc` - sRGB v2 (2014)
- `sRGB-v4-ICC_preference.icc` - sRGB v4 preference
- `Display P3.icc` - Display P3
- `LargeRGB-elle-V2-g18.icc` - ProPhoto RGB variant
- `LargeRGB-elle-V4-g18.icc` - ProPhoto RGB variant v4
- `CIERGB-elle-V2-labl.icc` - CIE RGB v2
- `CIERGB-elle-V4-labl.icc` - CIE RGB v4
- `swapped-v2.icc` - R/G swapped profile for obvious detection

## iccmax/ (ICC iccMAX Profiles)

**Source:** https://github.com/InternationalColorConsortium/DemoIccMAX
**License:** ICC Software License (BSD-style)
**Copyright:** (c) International Color Consortium

iccMAX (ICC v5) profiles and spectral reference data:
- `LCDDisplayCat8Obs.icc` - LCD display with CAT08 observer model
- `sRGB_v4_ICC_preference.icc` - sRGB v4 preference

### iccmax/named-color-xml/
iccMAX Named Color profile XML definitions (require iccMAX tools to compile):
- `NamedColor.xml` - Extended named color example with spectral data
- `FluorescentNamedColor.xml` - Fluorescent named color profile
- `SparseMatrixNamedColor.xml` - Sparse matrix named color profile

## Root Directory

Additional reference profiles (MIT/BSD licensed from various sources):
- `sRGB.icc` - Standard sRGB
- `AdobeRGB1998.icc` - Adobe RGB (1998)
- `DisplayP3.icc` - Display P3
- `Rec2020.icc` - ITU-R BT.2020

## Named Color and Spot Color Profiles

For testing named color (nmcl class) and spot color support:

### lcms2/crayons.icc
A multilingual RGB profile used for testing MLU (Multi-Localized Unicode) support,
not actually a named color profile despite the name.

### CMYK Profiles for Testing
- `skcms/misc/Coated_FOGRA39_CMYK.icc` - Standard FOGRA39 CMYK (654KB, full A2B/B2A LUTs)
- `qcms/ps_cmyk_min.icc` - Minimal CMYK profile
- `lcms2/test1.icc` - Little CMS CMYK test profile (Lab PCS)
- `lcms2/plugins/test2.icc` - Little CMS CMYK test profile (Lab PCS)
