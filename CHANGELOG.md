# Changelog

All notable changes to this workspace are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). None of the crates is
published yet.

## [Unreleased]

### QUEUED BREAKING CHANGES

- None.

### Changed

- Dependencies: thiserror 2, rand / rand_chacha 0.10, multiversion 0.8, criterion 0.7; the benches take `black_box` from `std::hint` (2010224). Imprecise floors raised to the resolved versions, all within the 1.89 MSRV (50f812b). Held by the MSRV: criterion 0.8, multiversion 0.9, colorutils-rs 0.8.
- CI: checkout v7, cache v6, upload-artifact v7 (89c5d61).
- `.workongoing` is no longer tracked (c2ebbd9).

### Fixed

- The test targets compile again: the vendored moxcms tests import rand 0.10's `RngExt` (35bb136), and `named_color_profiles` passes `profiles_dir` by reference (f7c3a8f).

### Known Bugs

- CI cannot build `argyll-sys`: it compiles `external/argyllcms/icc/*.c`, which is gitignored and never fetched by the workflow.
- `cargo test --all --no-fail-fast` fails 7 tests: the lcms2-vs-moxcms CMYK parity tolerance (2), the moxcms pure-yellow regression tests (4, green channel off by 6-8), and `grayscale_corpus_parsing_and_transforms` (lcms2 output not monotonic for `direct_fit_negative_a_fb0f7fbd.icc`).
