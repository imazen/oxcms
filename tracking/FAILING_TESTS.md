# Failing Tests Tracker

This document tracks all failing tests, their root cause, and remediation plan.

## Summary

| Category | Count | Blocking Release |
|----------|-------|------------------|
| Critical | 0 | Yes |
| High | 0 | Yes |
| Medium | 0 | No |
| Low | 0 | No |
| **Total** | **0** | - |

---

## Critical Failures

*Tests that produce completely wrong output or crash.*

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
