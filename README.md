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

**Work in Progress** - Currently in Phase 1 (Test Infrastructure)

- [x] Project structure
- [x] Parity test framework (moxcms vs lcms2)
- [x] Math difference documentation
- [x] CI workflow
- [ ] Port moxcms implementation
- [ ] Profile-to-profile transforms
- [ ] CMYK support
- [ ] DeviceLink profiles

## Quick Start

```bash
# Fetch test ICC profiles
./scripts/fetch-test-profiles.sh

# Run all tests
cargo test --all

# Run parity tests with output
cargo test -p cms-tests -- --nocapture

# Run specific test
cargo test -p cms-tests math_differences -- --nocapture
```

## Project Structure

```
oxcms/
├── crates/
│   ├── oxcms-core/     # Main CMS implementation
│   └── cms-tests/      # Cross-CMS parity tests
├── testdata/
│   ├── profiles/       # Standard ICC profiles
│   └── corpus/         # Test profiles from each CMS
├── docs/               # Architecture, math differences
├── plans/              # Roadmap, phase plans
└── tracking/           # Test status, failing tests
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](plans/ROADMAP.md)
- [Math Differences](docs/MATH_DIFFERENCES.md)
- [Test Status](tracking/TEST_STATUS.md)

## Other CMS Libraries Considered

| Library | Language | Status |
|---------|----------|--------|
| moxcms | Rust | Primary reference |
| lcms2 | C | Accuracy reference |
| skcms | C | Security model reference |
| qcms | Rust | Firefox's CMS |
| rcms | Rust | GPU-friendly, experimental |

## AI-Generated Code Notice

This project was developed with assistance from Claude (Anthropic). Not all code has been manually reviewed. Validate independently before production use.

## License

MIT OR Apache-2.0

## Sources

- [moxcms](https://github.com/awxkee/moxcms)
- [LittleCMS](https://www.littlecms.com/)
- [skcms](https://skia.googlesource.com/skcms/)
- [qcms](https://github.com/FirefoxGraphics/qcms)
