# oxcms - Oxidized Color Management System

A Rust color-management wrapper and cross-CMS test workspace.

## Goals

Combine the best of all CMS implementations:

| Source | Contribution |
|--------|-------------|
| **moxcms** | Rust safety, SIMD performance, modern API |
| **lcms2** | Full ICC v4.4, CMYK, DeviceLink, CIECAM02 |
| **skcms** | OSS-Fuzz hardening, HDR (PQ/HLG) |
| **qcms** | Firefox battle-tested reliability |

## Status

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1: Test Infrastructure | Available | Parity tests and documentation; known CI/test failures remain |
| Phase 2: Core Implementation | In Progress | See "Relationship with moxcms" below |
| Phase 3: Feature Parity | Planned | CMYK, DeviceLink, advanced features |
| Phase 4: Beyond lcms2 | Planned | HDR, fuzzing, profile creation |

### Relationship with moxcms

oxcms currently wraps [moxcms](https://github.com/awxkee/moxcms), an excellent Rust CMS by [Radzivon Bartoshyk](https://github.com/awxkee). We're actively contributing bug fixes upstream:

- [PR #139](https://github.com/awxkee/moxcms/pull/139) - ARM64 NEON register fix
- [PR #140](https://github.com/awxkee/moxcms/pull/140) - V2 ICC white point handling
- [PR #141](https://github.com/awxkee/moxcms/pull/141) - Flexible version parsing

**The long-term approach is undecided.** We're evaluating:
1. **Continue wrapping** - Maintain oxcms as a stable API layer over moxcms
2. **Deeper collaboration** - Contribute features directly to moxcms
3. **Independent implementation** - Build from scratch if our requirements diverge

We prefer options 1 or 2. The goal is to improve the Rust color management ecosystem, not fragment it.

### Phase 1 Achievements

- [x] Workspace with oxcms-core, cms-tests, skcms-sys
- [x] Cross-CMS test infrastructure (known failures: [test status](tracking/TEST_STATUS.md))
- [x] Cross-CMS comparison (moxcms, lcms2, qcms, skcms)
- [x] DeltaE2000 accuracy measurement
- [x] CI configuration for Ubuntu, Windows and macOS (see known failures below)
- [x] ARM64 NEON bug identified and fixed in moxcms fork
- [x] Math differences documented

### Current Implementation

`oxcms-core` provides a stable API layer:
- Clean public types that don't leak moxcms internals
- The top-level wrapper delegates transforms to vendored moxcms 0.8.0; native modules are also present
- Additional validation and error handling

## Quick Start

```bash
# Run all tests
cargo test --all

# Run parity tests with output
cargo test -p cms-tests -- --nocapture

# Run specific test category
cargo test -p cms-tests lcms2_parity -- --nocapture
cargo test -p cms-tests math_differences -- --nocapture
```

## Example Usage

```rust
use oxcms_core::{ColorProfile, Layout, TransformOptions};

// Create profiles
let srgb = ColorProfile::new_srgb();
let p3 = ColorProfile::new_display_p3();

// Create transform
let transform = srgb.create_transform_8bit(
    Layout::Rgb,
    &p3,
    Layout::Rgb,
    TransformOptions::default(),
).unwrap();

// Transform pixels
let src = [255u8, 128, 64];
let mut dst = [0u8; 3];
transform.transform(&src, &mut dst).unwrap();
```

## Project Structure

```
oxcms/
├── crates/
│   ├── oxcms-core/     # Main CMS implementation (wraps moxcms)
│   ├── cms-tests/      # Cross-CMS parity tests
│   └── skcms-sys/      # FFI bindings to skcms
├── external/
│   └── moxcms/         # Forked moxcms with ARM64 fix
├── testdata/
│   └── corpus/         # 121 ICC test profiles
├── docs/               # Architecture, math differences
├── plans/              # Roadmap, implementation plan
└── tracking/           # Test status
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - Design and structure
- [Roadmap](plans/ROADMAP.md) - Implementation phases
- [Implementation Plan](plans/IMPLEMENTATION_PLAN.md) - Detailed approach
- [Math Differences](docs/MATH_DIFFERENCES.md) - CMS comparison results
- [Test Status](tracking/TEST_STATUS.md) - Current test coverage

## Validation and limitations

The full workspace is not all-green. The last recorded full run had seven
failures (CMYK parity, pure yellow and a grayscale monotonicity case), and
CI requires external Argyll C sources that its workflow does not fetch.
See [CHANGELOG](CHANGELOG.md) and [test status](tracking/TEST_STATUS.md).

The [2026-11-17 ICC comparison](docs/ICC_DEVIATIONS.md) tests current
upstream moxcms 0.9.2 separately from the wrapper's vendored backend.
It records shared and different behavior for the fifteen lcms2.20 findings,
including unsupported operations, metadata loss and native oxcms curve issues.
Passing parity checks does not establish specification conformance.

```bash
just icc-audit
just icc-audit-extended
```

The standalone audit has a pinned upstream revision and lockfile. These
commands do not require Argyll or certify the ordinary workspace suite.

## CMS Libraries Compared

| Library | Language | Status |
|---------|----------|--------|
| moxcms | Rust | Primary reference, forked with fixes |
| lcms2 | C | Accuracy reference |
| skcms | C++ | Security model reference |
| qcms | Rust | Firefox's CMS |

## AI-Generated Code Notice

This project was developed with assistance from Claude (Anthropic). The code has cross-CMS tests, but **not all code has been manually reviewed**, and known failures remain.

Before using in production:
- Review critical code paths for your use case
- Run your own validation against expected outputs
- Consider the test suite coverage for your specific requirements

## License

MIT OR Apache-2.0

## Sources & Acknowledgments

- [moxcms](https://github.com/awxkee/moxcms) by [Radzivon Bartoshyk](https://github.com/awxkee) - Fast Rust CMS we build upon
- [LittleCMS](https://www.littlecms.com/) - Industry standard reference
- [skcms](https://skia.googlesource.com/skcms/) - Chrome's CMS
- [qcms](https://github.com/nicholasbishop/qcms-rust) - Firefox's CMS
