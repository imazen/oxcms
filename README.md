# oxcms - Oxidized Color Management System

A fast, safe, and complete color management system in Rust.

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
| Phase 1: Test Infrastructure | **Complete** | Parity tests, CI, documentation |
| Phase 2: Core Implementation | In Progress | See "Relationship with moxcms" below |
| Phase 3: Feature Parity | Planned | CMYK, DeviceLink, advanced features |
| Phase 4: Beyond lcms2 | Planned | HDR, fuzzing, profile creation |

### Relationship with moxcms

oxcms currently wraps [moxcms](https://github.com/awxkee/moxcms), an excellent Rust CMS by [@awxkee](https://github.com/awxkee). We're actively contributing bug fixes upstream:

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
- [x] 185 parity tests (all passing)
- [x] Cross-CMS comparison (moxcms, lcms2, qcms, skcms)
- [x] DeltaE2000 accuracy measurement
- [x] CI on Ubuntu, Windows, macOS (x86_64 + ARM64)
- [x] ARM64 NEON bug identified and fixed in moxcms fork
- [x] Math differences documented

### Current Implementation

`oxcms-core` provides a stable API layer:
- Clean public types that don't leak moxcms internals
- All transforms currently delegated to moxcms
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

## Test Results

All CMS implementations produce **identical output** for sRGB transforms:

| Comparison | Max ΔE | Status |
|------------|--------|--------|
| moxcms vs lcms2 | 0.0000 | IDENTICAL |
| moxcms vs qcms | 0.0000 | IDENTICAL |
| qcms vs lcms2 | 0.0000 | IDENTICAL |

## Performance Targets

| Operation | Target | Reference |
|-----------|--------|-----------|
| sRGB→sRGB 1MP | < 1ms | moxcms baseline |
| sRGB→P3 1MP | < 2ms | moxcms baseline |
| ICC parse | < 100μs | moxcms baseline |

Goal: Match or exceed moxcms performance (3x+ faster than lcms2).

## CMS Libraries Compared

| Library | Language | Status |
|---------|----------|--------|
| moxcms | Rust | Primary reference, forked with fixes |
| lcms2 | C | Accuracy reference |
| skcms | C++ | Security model reference |
| qcms | Rust | Firefox's CMS |

## AI-Generated Code Notice

This project was developed with assistance from Claude (Anthropic). While the code has been tested against multiple reference implementations and passes 185+ tests including cross-CMS parity validation, **not all code has been manually reviewed**.

Before using in production:
- Review critical code paths for your use case
- Run your own validation against expected outputs
- Consider the test suite coverage for your specific requirements

## License

MIT OR Apache-2.0

## Sources & Acknowledgments

- [moxcms](https://github.com/awxkee/moxcms) by [@awxkee](https://github.com/awxkee) - Fast Rust CMS we build upon
- [LittleCMS](https://www.littlecms.com/) - Industry standard reference
- [skcms](https://skia.googlesource.com/skcms/) - Chrome's CMS
- [qcms](https://github.com/nicholasbishop/qcms-rust) - Firefox's CMS
