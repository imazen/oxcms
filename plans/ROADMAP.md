# oxcms Roadmap

## Overview

oxcms aims to be the definitive Rust color management system, combining:
- moxcms speed and safety
- lcms2 completeness
- skcms security hardening
- qcms battle-tested reliability

## Phase 1: Test Infrastructure (Week 1-2)

### Goals
- [ ] Clone and analyze moxcms source
- [ ] Clone and analyze lcms2 testbed
- [ ] Clone and analyze skcms profiles/tests
- [ ] Clone and analyze qcms tests
- [ ] Set up parity test framework
- [ ] Document all math differences

### Deliverables
- Working test suite comparing all 4 implementations
- `docs/MATH_DIFFERENCES.md` documenting every divergence
- CI running all comparison tests

### Test Sources

| Library | Test Location | Key Files |
|---------|--------------|-----------|
| moxcms | `tests/` | Unit tests |
| lcms2 | `testbed/testcms2.c` | 400+ tests |
| lcms2 | `testbed/*.icc` | Test profiles |
| skcms | `profiles/` | Reference profiles |
| skcms | OSS-Fuzz corpus | Fuzzing corpus |
| qcms | `tests/` | Firefox integration tests |

## Phase 2: Achieve Parity (Week 3-6)

### Goals
- [ ] All lcms2 testbed tests passing
- [ ] All skcms profile tests passing
- [ ] All qcms tests passing
- [ ] No unexplained math differences

### Priority Order
1. sRGB transforms (most common)
2. Display P3 / Wide gamut RGB
3. CMYK transforms
4. Lab/XYZ transforms
5. DeviceLink profiles
6. Named color profiles

## Phase 3: Extend (Week 7+)

### New Features
- [ ] Profile creation API
- [ ] CIECAM02 appearance model
- [ ] Black point compensation options
- [ ] HDR transfer functions (PQ, HLG)
- [ ] GPU-friendly pipeline exposure (like rcms)

### Security
- [ ] OSS-Fuzz integration
- [ ] cargo-fuzz harnesses
- [ ] Memory safety audits
- [ ] Malformed profile handling

## Success Criteria

### Performance
- At least as fast as moxcms (3x+ faster than lcms2)
- SIMD paths for AVX2, SSE4, NEON

### Accuracy
- DeltaE2000 < 0.0001 vs lcms2 for all standard transforms
- Bit-exact where possible

### Completeness
- Full ICC v4.4 support
- All lcms2 testbed tests pass
- All skcms profile tests pass

### Safety
- Zero unsafe code in hot paths (or justified and audited)
- No panics on malformed input
- Fuzzing corpus with 100% coverage of parse paths

## Timeline

```
Week 1-2:  [==========] Test Infrastructure
Week 3-4:  [==========] RGB Parity
Week 5-6:  [==========] CMYK Parity
Week 7-8:  [==========] Advanced Features
Week 9+:   [==========] Polish & Security
```

## Non-Goals (For Now)

- GPU compute shaders
- Embedded systems (no_std)
- WebAssembly target
- FFI bindings (C API)

These can be added later but are not in initial scope.
