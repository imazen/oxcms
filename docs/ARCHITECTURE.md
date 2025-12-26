# oxcms Architecture

## Goal

Create the definitive Rust color management system by combining the best aspects of:

| Library | Strengths to Incorporate |
|---------|-------------------------|
| **moxcms** | Rust safety, SIMD performance, modern API |
| **lcms2** | Full ICC v4.4, CMYK, DeviceLink, CIECAM02 |
| **skcms** | Fuzzing infrastructure, HDR (PQ/HLG), Chrome-hardened |
| **qcms** | Firefox battle-tested, pure Rust |

## Project Structure

```
oxcms/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── oxcms-core/          # Main CMS implementation (fork of moxcms)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── profile.rs        # ICC profile parsing
│   │   │   ├── transform.rs      # Color transforms
│   │   │   ├── cmyk.rs           # CMYK support
│   │   │   ├── lab.rs            # L*a*b* color space
│   │   │   ├── lut.rs            # Lookup tables
│   │   │   ├── simd/             # SIMD implementations
│   │   │   │   ├── mod.rs
│   │   │   │   ├── avx2.rs
│   │   │   │   ├── sse4.rs
│   │   │   │   └── neon.rs
│   │   │   └── icc/              # ICC tag parsing
│   │   │       ├── mod.rs
│   │   │       ├── v2.rs         # ICC v2 support
│   │   │       └── v4.rs         # ICC v4 support
│   │   └── Cargo.toml
│   │
│   └── cms-tests/                # Test infrastructure
│       ├── src/
│       │   ├── lib.rs
│       │   ├── parity.rs         # Cross-library parity tests
│       │   ├── accuracy.rs       # DeltaE accuracy measurement
│       │   └── corpus.rs         # Test corpus management
│       ├── tests/
│       │   ├── skcms_parity.rs   # skcms comparison tests
│       │   ├── lcms2_parity.rs   # lcms2 comparison tests
│       │   ├── qcms_parity.rs    # qcms comparison tests
│       │   └── math_diff.rs      # Document all math differences
│       └── Cargo.toml
│
├── testdata/
│   ├── profiles/                 # ICC profiles for testing
│   │   ├── srgb/
│   │   ├── display-p3/
│   │   ├── adobe-rgb/
│   │   ├── cmyk/
│   │   └── exotic/               # Edge cases, malformed, etc.
│   ├── corpus/                   # Fuzzing corpus
│   │   ├── skcms/
│   │   ├── lcms2/
│   │   └── qcms/
│   └── reference-outputs/        # Expected outputs from reference implementations
│       ├── lcms2/
│       ├── skcms/
│       └── qcms/
│
├── docs/
│   ├── ARCHITECTURE.md           # This file
│   ├── MATH_DIFFERENCES.md       # Document all math differences
│   ├── ICC_SPEC_NOTES.md         # ICC specification notes
│   └── PERFORMANCE.md            # Performance analysis
│
├── plans/
│   ├── ROADMAP.md                # High-level roadmap
│   ├── PHASE_1_TESTS.md          # Phase 1: Test infrastructure
│   ├── PHASE_2_PARITY.md         # Phase 2: Achieve parity
│   └── PHASE_3_EXTEND.md         # Phase 3: Add new features
│
├── tracking/
│   ├── TEST_STATUS.md            # Status of all tests
│   ├── FAILING_TESTS.md          # Known failing tests
│   ├── IMPLEMENTATION_LOG.md     # Implementation progress
│   └── DECISIONS.md              # Design decisions and rationale
│
├── scripts/
│   ├── fetch-test-profiles.sh    # Download test ICC profiles
│   ├── generate-reference.py     # Generate reference outputs
│   └── compare-outputs.py        # Compare CMS outputs
│
└── .github/
    └── workflows/
        ├── ci.yml                # Main CI workflow
        ├── fuzz.yml              # Fuzzing workflow
        └── benchmarks.yml        # Performance benchmarks
```

## Implementation Phases

### Phase 1: Test Infrastructure (Current)
- Set up comprehensive test corpus from all libraries
- Create parity test framework
- Document existing math differences

### Phase 2: Achieve Parity
- Fix all failing tests
- Match lcms2 output for all documented cases
- Add missing ICC tag support

### Phase 3: Extend
- Add DeviceLink profiles
- Implement CIECAM02
- Add profile creation
- HDR support (PQ/HLG)

## Key Design Decisions

See `tracking/DECISIONS.md` for detailed rationale.

### 1. Start from moxcms
- Already Rust, memory-safe
- Good SIMD foundation
- Active development

### 2. lcms2 as Reference
- Most complete ICC implementation
- Industry standard
- Extensive test suite

### 3. All Differences Documented
- No silent divergence from reference
- Every math difference justified
- Parity tests enforce consistency
