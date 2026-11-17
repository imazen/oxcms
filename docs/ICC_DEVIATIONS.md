# ICC deviations: moxcms, lcms2 and oxcms

Checked 2026-11-17 against upstream moxcms's default branch, **master**,
at [`5fc602b66fefeb40094a4adc7eea965aae0c28ea`](https://github.com/awxkee/moxcms/tree/5fc602b66fefeb40094a4adc7eea965aae0c28ea)
(package 0.9.2). The wrapper still uses vendored 0.8.0. The comparison does
not upgrade that dependency or change either implementation's numerical contract.

The D1–D15 identifiers refer to the lcms2.20 conformance audit. This is a
bounded comparison of those findings, not a general conformance certificate.
Unsupported operations and discarded tags do **not** count as conformance.

## Reproduce

`tools/icc-audit` is a separate Cargo workspace, with upstream moxcms pinned
by git revision and a committed lockfile. It also probes oxcms's native
curve code. It avoids `cms-tests`' external Argyll build requirement.

```
just icc-audit
just icc-audit-extended
just icc-audit-lcms /path/to/lcms2
```

The recipes use the shared `run-heavy` wrapper. The C recipe needs a C
compiler and an explicit lcms source directory, and compiles only into the
audit's ignored target directory. This run used unmodified tracked C source
at `7717fce0d85447ef3b2df0cca2e99207c15eeecd`; the probe reported encoded
CMM version **2200**. Existing untracked testbed profiles were not inputs.
No downloaded ICC fixtures or diagnostic images are required.

## All fifteen findings

| ID | lcms2.20 finding | Current moxcms behavior | Assessment |
|---|---|---|---|
| D1 | Transform-generated DeviceLinks lack `pseq` without KEEP_SEQUENCE | No equivalent transform-to-DeviceLink generator or `pseq` writer | Unsupported operation; not a conformance pass |
| D2 | NULL-profile generator emits an incomplete Output profile | No equivalent NULL-profile generator | Unsupported operation |
| D3 | D65 gray Display profile stores unadapted `wtpt` | `new_gray_with_gamma` writes D50 `wtpt` and a D65→D50 `chad` | Differs; follows the v4 display-white requirement |
| D4 | v2 `desc` sizes include padding | Writer upgrades v2 to v4 and writes descriptions as `mluc`; the tested 30-byte tag excludes alignment padding | Different writer scope; no v2 output to judge |
| D5 | `psid` internal structures lack alignment | `psid` is not retained by parse→encode | Unsupported metadata; round-trip loss |
| D6 | Saving v4 as v2 retains v4-only tags | No v2 output mode; parsed v2 is encoded as v4 | No equivalent version downgrade |
| D7 | Curves-only pipeline exported as `lut16` has a zero-grid CLUT | A caller-supplied `LutDataType` with zero grid points is emitted as `mft2`, then rejected by moxcms's own parser | Same invalid structure is possible; not the same automatic generation path |
| D8 | Internal `FFFF` locale escapes into `mluc` | Legacy `desc` becomes `enUS` `mluc`; no lcms-style MLU sequence conversion | Does not reproduce the tested conversion; arbitrary locale validation not certified |
| D9 | Explicit MD5 computation fills v2-reserved bytes | No equivalent ID computation or v2 writer; parse→encode zeros the ID and emits v4 | No equivalent operation |
| D10 | v4 ID stays zero without explicit computation | Encoder always writes a zero ID | Replicates absence of an ID; this is a **should**, not a **shall** |
| D11 | `dict` components lack alignment | `dict` metadata is discarded | Unsupported metadata; not a conformant writer |
| D12 | `dict` embeds bare `mluc` bodies | No `dict` writer; metadata is discarded | Unsupported metadata; no interoperable round-trip |
| D13 | Direct float parametric evaluation is unclipped | Direct evaluator also returns 1.5 and −0.25; default RGB float transform clips; explicit extended-range transform preserves 1.5 | Replicates direct-evaluator behavior, with a bounded default transform path |
| D14 | Plateau inverse does not use Annex F.1 endpoints | Also differs from F.1 in some cases, but results are not bit-exact with lcms | Shared issue class, different answers; see below |
| D15 | 44-byte amended `cicp` rejected | Accepts it as Unspecified primaries, but keeps only the first 12 bytes and re-encodes those | More tolerant reader; custom coordinates are lost, not supported |

D3–D15 above have executable witnesses where an equivalent API exists.
D1/D2 and the absence of sequence/dictionary writers were checked against
`ColorProfile`, the defaults, parser dispatch and encoder dispatch. Empty
`psid`/`dict` witnesses demonstrate metadata loss, not every nonempty layout.

Source: moxcms [`defaults.rs`](https://github.com/awxkee/moxcms/blob/5fc602b66fefeb40094a4adc7eea965aae0c28ea/src/defaults.rs),
[`writer.rs`](https://github.com/awxkee/moxcms/blob/5fc602b66fefeb40094a4adc7eea965aae0c28ea/src/writer.rs),
[`reader.rs`](https://github.com/awxkee/moxcms/blob/5fc602b66fefeb40094a4adc7eea965aae0c28ea/src/reader.rs),
[`trc.rs`](https://github.com/awxkee/moxcms/blob/5fc602b66fefeb40094a4adc7eea965aae0c28ea/src/trc.rs).

## What the inverse witnesses actually show

4096-entry forward tables, with values rounded to uint16:

| Curve / query | Annex F.1 endpoint rule | lcms2.20, 4096-entry inverse | moxcms `inverse()`, 4096 entries | moxcms `make_gamma_evaluator()`, 16384 entries | Native oxcms direct inverse |
|---|---|---|---|---|---|
| Interior plateau x=0.3…0.6; y=32768/65535 | Highest plateau x, approximately 0.6 | 0.459418625 | 0.459412932 | 0.487544835 | 0.300122100 |
| End plateau x=0.5…1; y=0.8 | Lowest plateau x, approximately 0.5 | 1.000000000 | 0.749996185 | 0.700245619 | 0.500122100 |
| End plateau x=0.5…1; y=1 | Lowest plateau x, approximately 0.5 | 1.000000000 | 0.500007629 | 0.500007629 | Not probed |

The endpoint is approximate here because the **forward** curve is sampled.
For the interior plateau, ideal decimal 0.5 is not the stored plateau value:
the moxcms 4096-entry inverse returns **0.450034350** at decimal 0.5 and
**0.459412932** at 32768/65535. Exact query values matter.

Resolution matters too. With 65530 forward and inverse entries, lcms gives
0.600000024 for the interior case and 0.800000012 for the end-at-0.8 case.
With 65536 entries, moxcms gives 0.499992371 and 0.749996185 respectively.
Those larger tables use different sizes because lcms caps curves at 65530;
they are sensitivity checks, not matched comparisons.

Annex F.1 defines a deterministic generalized inverse. It necessarily has
a jump at a nontrivial plateau; a uniformly sampled inverse with linear
interpolation cannot represent that jump exactly at every possible ordinate.
This explains why an approximation can miss the endpoint, but does not
make the normative endpoint rule wrong. The different treatment of a plateau
ending at 1 between lcms and moxcms is a concrete implementation difference.

## Specification questions and oxcms work

There is **no demonstrated specification error** from agreement between
these libraries. The strongest clarification question is how F.1's exact
plateau choice should be validated for sampled approximate inverses. A
test should identify the actual encoded plateau, inverse resolution and
interpolation, rather than compare an ideal decimal input alone.

Clipping is a separate API distinction. ICC.1:2022 10.18 requires bounded
`parametricCurveType` output. General-purpose unbounded evaluators and an
explicit extended-range mode serve a different contract. The audit's RGB
float fixture returns `[1,1,1]` with defaults in both builds; with the
`extended_range` feature **and** `allow_extended_range_rgb_xyz=true`, it
returns `[1.5,1.5,1.5]`. This does not certify every transform route.

Native oxcms findings, independent of the vendored wrapper:

- `math::gamma::parametric_curve_eval` clamps input but not output: it returns
  1.5 for `(2x)^1` at x=0.75. Its ICC semantics need an explicit bounded
  path; do not silently change a general extended-range API.
- `icc::tags::CurveData::eval_inverse` picks the low edge of an interior
  plateau, rather than F.1's high edge. It picks the expected low edge of
  the tested end plateau. These observations do not establish behavior for
  descending or nonmonotonic curves.
- The upstream encoder can emit a zero-grid `lut16` it cannot reopen, and
  discards extended CICP coordinates and unsupported metadata. A future
  wrapper/backend update needs explicit preservation/rejection policy.

These are follow-up implementation decisions. This change records witnesses
and findings without upgrading the production backend, changing public API,
or weakening the existing parity tests. Existing CMYK qualification work
is tracked separately in oxcms PR 4.

## Sources and validation

- [ICC.1:2022-05](https://www.color.org/specifications/ICC.1-2022-05.pdf):
  7.2.18 (ID), 9.2.36 (white point), 10.18/Table 68 (curve clipping),
  Annex F.1 (inverse conventions), and the tag-layout clauses in D1–D12.
- [CICP amendment, approved 2026-10-19](https://media.color.org/icc1_amendment_cicptype.pdf):
  optional custom chromaticities for ColourPrimaries=2. Its compatibility
  discussion permits older readers to retain Unspecified handling; merely
  accepting 44 bytes is not evidence that a reader uses the coordinates.
- [Current ICC v4 specification and amendments](https://www.color.org/v4spec/).

The audit tests pass with the default and extended-range feature sets, and
the C probe completes against lcms2.20. The ordinary workspace suite was
not rerun: its previously recorded failures and external Argyll build
requirement remain; see [CHANGELOG](../CHANGELOG.md) and
[test status](../tracking/TEST_STATUS.md).
