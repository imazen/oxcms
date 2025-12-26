# Design Decisions Log

This document records all significant design decisions and their rationale.

## Decision Format

```markdown
### DEC-XXX: Decision Title

**Date**: YYYY-MM-DD
**Status**: Proposed / Accepted / Superseded by DEC-YYY

**Context**:
What is the issue that we're addressing?

**Options Considered**:
1. Option A - description
2. Option B - description
3. Option C - description

**Decision**:
What option was chosen and why.

**Consequences**:
What becomes easier or harder as a result.
```

---

## Accepted Decisions

### DEC-001: Use moxcms as Foundation

**Date**: 2025-12-25
**Status**: Accepted

**Context**:
Need to choose a starting point for the unified CMS implementation.

**Options Considered**:
1. Start from scratch - Maximum control, maximum effort
2. Fork moxcms - Already Rust, good SIMD, active development
3. Fork qcms - Firefox battle-tested, but limited features
4. Wrap lcms2 - Complete but not memory-safe

**Decision**:
Fork moxcms as the foundation.

**Rationale**:
- Already pure Rust (memory-safe by design)
- Excellent SIMD implementation (AVX2, SSE4, NEON)
- 3-5x faster than lcms2 in benchmarks
- Active development and responsive maintainer
- Modern Rust API design

**Consequences**:
- Need to add missing features (DeviceLink, CIECAM02, profile creation)
- Must maintain parity tests against lcms2
- Can leverage existing SIMD infrastructure

---

### DEC-002: lcms2 as Reference Implementation

**Date**: 2025-12-25
**Status**: Accepted

**Context**:
Need a reference implementation for correctness testing.

**Options Considered**:
1. lcms2 - Industry standard, full ICC support
2. skcms - Chrome-hardened, but RGB only
3. ICC reference implementation - Theoretical but not practical

**Decision**:
Use lcms2 as the primary reference for correctness.

**Rationale**:
- Full ICC v4.4 implementation
- Industry standard (used in GIMP, Inkscape, etc.)
- Extensive test suite (400+ tests in testbed)
- Covers CMYK, DeviceLink, NamedColor
- Well-documented behavior

**Consequences**:
- Any divergence from lcms2 must be documented and justified
- Need to port lcms2 testbed to Rust
- May need lcms2 FFI bindings for comparison

---

### DEC-003: Separate Test Crate

**Date**: 2025-12-25
**Status**: Accepted

**Context**:
Need to organize test infrastructure.

**Options Considered**:
1. Tests in main crate - Simple but mixes concerns
2. Separate test crate - Clean separation, reusable
3. Multiple test crates per source - Maximum isolation

**Decision**:
Single `cms-tests` crate for all integration tests.

**Rationale**:
- Keeps main crate focused on implementation
- Can depend on multiple CMS libraries for comparison
- Single place for all parity/accuracy tests
- Easier to maintain

**Consequences**:
- Need workspace for multi-crate setup
- Test crate has heavy dependencies (lcms2, etc.)
- Clear separation of unit vs integration tests

---

### DEC-004: Document All Math Differences

**Date**: 2025-12-25
**Status**: Accepted

**Context**:
Different CMS implementations may produce slightly different results.

**Options Considered**:
1. Ignore differences < threshold - Simple but hides issues
2. Document all differences - Transparent, more work
3. Enforce bit-exact - May be impossible

**Decision**:
Document every math difference in `docs/MATH_DIFFERENCES.md`.

**Rationale**:
- Transparency is critical for users
- Helps debug issues
- Enables informed decisions about tolerance
- Some differences are intentional (e.g., different rounding)

**Consequences**:
- More documentation overhead
- Need automated difference detection
- Users can make informed choices

---

## Proposed Decisions

(None currently pending)

---

## Superseded Decisions

(None yet)
