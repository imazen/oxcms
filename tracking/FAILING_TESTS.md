# Failing Tests Tracker

This document tracks all failing tests, their root cause, and remediation plan.

## Summary

| Category | Count | Blocking Release |
|----------|-------|------------------|
| Critical | 0 | Yes |
| High | 1 | Yes |
| Medium | 0 | No |
| Low | 0 | No |
| **Total** | **1** | - |

---

## Critical Failures

*Tests that produce completely wrong output or crash.*

(None)

---

## High Priority Failures

*Tests that affect common workflows but don't crash.*

### [CMYK-001] CMYK LUT interpolation differs from lcms2

**Source**: moxcms vs lcms2
**File**: `tests/cmyk_parity.rs`
**Status**: 🔴 Failing (partially diagnosed)

**Description**:
CMYK transforms using LUT-based profiles (USWebCoatedSWOP) produce different results than lcms2.

**Observed (with trilinear interpolation - moxcms default)**:
- CMYK→RGB: max_diff=7, 32 cases >1 diff
- RGB→CMYK: max_diff=4, 343 cases >1 diff

**Observed (with tetrahedral interpolation)**:
- CMYK→RGB: max_diff=7, **4 cases >1 diff** (improved!)
- RGB→CMYK: max_diff=4, 341 cases >1 diff (minimal change)

**Pattern (after tetrahedral fix)**:
- CMYK→RGB: Only 4 remaining cases, all on pure yellow axis:
  - `[0,0, 64,0]` → lcms2: G=251, moxcms: G=248 (diff 3)
  - `[0,0,128,0]` → lcms2: G=247, moxcms: G=242 (diff 5)
  - `[0,0,192,0]` → lcms2: G=244, moxcms: G=237 (diff 7)
  - `[224,128,192,64]` → diff 2
- RGB→CMYK: ~341 cases with diff 2-4, scattered across color space

**Root Cause**:
1. **Interpolation method** (trilinear vs tetrahedral) caused most CMYK→RGB diffs - FIXED by using tetrahedral
2. **Yellow axis boundary case** remains - differs by 3-7 in green channel
3. **RGB→CMYK** differences - ~341 cases with diff 2-4

**Analysis**:
Both lcms2 and moxcms use the same 4D approach:
- 3D tetrahedral on K=0 slice → Tmp1
- 3D tetrahedral on K=1 slice → Tmp2
- Linear interpolation between slices using K weight

**Cross-CMS verification** (pure yellow axis [0,0,Y,0], green channel):
| Y | lcms2 | mox(def) | mox(tet) | skcms | Δdef | Δtet | Δskcms |
|---|-------|----------|----------|-------|------|------|--------|
| 64 | 251 | 248 | 248 | 250 | 3 | 3 | 1 |
| 128 | 247 | 242 | 242 | 246 | 5 | 5 | 1 |
| 192 | 244 | 237 | 237 | 243 | 7 | 7 | 1 |
| 255 | 242 | 234 | 234 | 241 | 8 | 8 | 1 |

**Key findings**:
1. skcms (Chrome) and lcms2 agree (Δ ≤1)
2. **moxcms is the outlier** (Δ 3-8)
3. **Tetrahedral vs trilinear makes NO difference** for pure yellow
4. Bug is NOT in interpolation method - it's elsewhere in moxcms pipeline

**Fix Plan**:
1. ✅ Use tetrahedral interpolation (fixed other cases, not this one)
2. ✅ Verified same algorithm structure as lcms2
3. ✅ Cross-CMS verification shows moxcms is outlier
4. **Next**: File upstream bug report to moxcms - bug is in CLUT handling, not interpolation

**Current Workaround**: Use `InterpolationMethod::Tetrahedral` in TransformOptions

**Assigned**: Unassigned
**Target**: TBD

---

### Template
```markdown
### [TEST-ID] Test Name

**Source**: lcms2/skcms/qcms/moxcms
**File**: `tests/example_test.rs:42`
**Status**: 🔴 Failing

**Description**:
What the test does and why it's important.

**Expected**:
What should happen.

**Actual**:
What actually happens.

**Root Cause**:
Why it fails.

**Fix Plan**:
1. Step one
2. Step two

**Blocked By**: None / [OTHER-TEST-ID]
**Blocking**: [OTHER-TEST-ID] / None

**Assigned**: @username
**Target**: Phase X / Week Y
```

---

## High Priority Failures

*Tests that affect common workflows but don't crash.*

(None yet)

---

## Medium Priority Failures

*Tests for less common features or edge cases.*

(None yet)

---

## Low Priority Failures

*Nice-to-have features or exotic profiles.*

(None yet)

---

## Recently Fixed

| Test ID | Description | Fixed In | PR |
|---------|-------------|----------|-----|
| - | - | - | - |

---

## How to Add a Failing Test

1. Create a test in `crates/cms-tests/tests/`
2. Run the test to confirm it fails
3. Add an entry to this document
4. Commit with message: `test: add failing test for [description]`

## How to Mark a Test Fixed

1. Implement the fix
2. Confirm test passes
3. Move entry to "Recently Fixed" section
4. Commit with message: `fix: [TEST-ID] description`
