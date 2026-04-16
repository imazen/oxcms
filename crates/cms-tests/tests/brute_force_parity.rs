//! Brute-force CMS parity comparison across the full ICC profile corpus.
//!
//! Transforms a standardized u16 test ramp through moxcms, skcms, lcms2,
//! and ArgyllCMS for every parseable RGB profile, across:
//!   - 2 rendering intents: Perceptual, Relative Colorimetric
//!   - 2 moxcms interpolation methods: Linear (default), Tetrahedral
//!
//! Outputs a TSV report to `/mnt/v/output/oxcms/brute_force_parity.tsv`.
//! Run with: `cargo test -p cms-tests --test brute_force_parity -- --nocapture`

use std::collections::BTreeMap;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

// ── Test ramp ───────────────────────────────────────────────────────────

const RAMP_STEPS: usize = 64;

fn make_ramp_u16() -> Vec<u16> {
    let mut pixels = Vec::with_capacity(RAMP_STEPS * 5 * 3);
    // Gray ramp
    for i in 0..RAMP_STEPS {
        let v = ((i as f64 / (RAMP_STEPS - 1) as f64) * 65535.0) as u16;
        pixels.extend_from_slice(&[v, v, v]);
    }
    // Pure R/G/B ramps
    for ch in 0..3usize {
        for i in 0..RAMP_STEPS {
            let v = ((i as f64 / (RAMP_STEPS - 1) as f64) * 65535.0) as u16;
            let mut px = [0u16; 3];
            px[ch] = v;
            pixels.extend_from_slice(&px);
        }
    }
    // Mixed hues (3D interior probing)
    for i in 0..RAMP_STEPS {
        let t = i as f64 / (RAMP_STEPS - 1) as f64;
        let r = ((t * 0.9 + 0.05) * 65535.0) as u16;
        let g = (((1.0 - t) * 0.8 + 0.1) * 65535.0) as u16;
        let b = ((((t * 2.0) % 1.0) * 0.7 + 0.15) * 65535.0) as u16;
        pixels.extend_from_slice(&[r, g, b]);
    }
    pixels
}

fn num_pixels(ramp: &[u16]) -> usize {
    ramp.len() / 3
}

fn max_channel_diff(a: &[u16], b: &[u16]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs())
        .max()
        .unwrap_or(0)
}

fn mean_channel_diff(a: &[u16], b: &[u16]) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    let sum: u64 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i64 - y as i64).unsigned_abs())
        .sum();
    sum as f64 / a.len() as f64
}

// ── Profile collection ──────────────────────────────────────────────────

fn collect_all_profiles() -> Vec<PathBuf> {
    let mut profiles = Vec::new();

    // Primary corpus: ~/.cache/zenpixels-icc/
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/lilith".into());
    let icc_cache = PathBuf::from(&home).join(".cache/zenpixels-icc");
    walk_icc(&icc_cache, &mut profiles);

    // Secondary: testdata/profiles/
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
        .join("profiles");
    walk_icc(&testdata, &mut profiles);

    profiles.sort();
    profiles.dedup();
    profiles
}

fn walk_icc(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_icc(&path, out);
            } else if path
                .extension()
                .is_some_and(|e| e == "icc" || e == "icm")
            {
                out.push(path);
            }
        }
    }
}

// ── CMS transform wrappers (u16) ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Intent {
    Perceptual,
    RelativeColorimetric,
}

impl std::fmt::Display for Intent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Intent::Perceptual => f.write_str("perceptual"),
            Intent::RelativeColorimetric => f.write_str("relcol"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Interp {
    Default,
    Tetrahedral,
}

impl std::fmt::Display for Interp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interp::Default => f.write_str("default"),
            Interp::Tetrahedral => f.write_str("tetrahedral"),
        }
    }
}

fn moxcms_transform_u16(
    icc_data: &[u8],
    ramp: &[u16],
    intent: Intent,
    interp: Interp,
) -> Option<Vec<u16>> {
    use moxcms::{
        BarycentricWeightScale, ColorProfile, InterpolationMethod, Layout, RenderingIntent,
        TransformOptions,
    };

    let src = ColorProfile::new_from_slice(icc_data).ok()?;
    let dst = ColorProfile::new_srgb();

    let opts = TransformOptions {
        rendering_intent: match intent {
            Intent::Perceptual => RenderingIntent::Perceptual,
            Intent::RelativeColorimetric => RenderingIntent::RelativeColorimetric,
        },
        allow_use_cicp_transfer: false,
        prefer_fixed_point: false,
        interpolation_method: match interp {
            Interp::Default => InterpolationMethod::Linear,
            Interp::Tetrahedral => InterpolationMethod::Tetrahedral,
        },
        barycentric_weight_scale: BarycentricWeightScale::High,
    };

    let t = src
        .create_transform_16bit(Layout::Rgb, &dst, Layout::Rgb, opts)
        .ok()?;
    let mut out = vec![0u16; ramp.len()];
    t.transform(ramp, &mut out).ok()?;
    Some(out)
}

fn lcms2_transform_u16(icc_data: &[u8], ramp: &[u16], intent: Intent) -> Option<Vec<u16>> {
    use lcms2::{Flags, Intent as LIntent, PixelFormat, Profile, Transform};

    let src = Profile::new_icc(icc_data).ok()?;
    let dst = Profile::new_srgb();

    let lcms_intent = match intent {
        Intent::Perceptual => LIntent::Perceptual,
        Intent::RelativeColorimetric => LIntent::RelativeColorimetric,
    };

    let flags = Flags::NO_OPTIMIZE | Flags::HIGHRES_PRECALC;
    let xform: Transform<[u16; 3], [u16; 3]> =
        Transform::new_flags(&src, PixelFormat::RGB_16, &dst, PixelFormat::RGB_16, lcms_intent, flags)
            .ok()?;

    let mut out = vec![0u16; ramp.len()];
    let src_px: &[[u16; 3]] = bytemuck::cast_slice(ramp);
    let dst_px: &mut [[u16; 3]] = bytemuck::cast_slice_mut(&mut out);
    xform.transform_pixels(src_px, dst_px);
    Some(out)
}

fn skcms_transform_u16(icc_data: &[u8], ramp: &[u16], intent: Intent) -> Option<Vec<u16>> {
    use skcms_sys::{skcms_AlphaFormat, skcms_PixelFormat};

    let priority: &[i32] = match intent {
        Intent::Perceptual => &[0, 1],
        Intent::RelativeColorimetric => &[1, 0],
    };

    let profile = skcms_sys::parse_icc_profile_with_priority(icc_data, priority)?;
    let srgb = skcms_sys::srgb_profile();

    let npix = num_pixels(ramp);
    let mut out = vec![0u16; ramp.len()];

    let ok = skcms_sys::transform_u16(
        ramp,
        skcms_PixelFormat::RGB_161616LE,
        skcms_AlphaFormat::Opaque,
        &profile,
        &mut out,
        skcms_PixelFormat::RGB_161616LE,
        skcms_AlphaFormat::Opaque,
        srgb,
        npix,
    );
    if ok { Some(out) } else { None }
}

fn argyll_transform_u16(icc_data: &[u8], ramp: &[u16], intent: Intent) -> Option<Vec<u16>> {
    let argyll_intent = match intent {
        Intent::Perceptual => argyll_sys::Intent::Perceptual,
        Intent::RelativeColorimetric => argyll_sys::Intent::RelativeColorimetric,
    };

    let npix = num_pixels(ramp);
    let mut out = vec![0u16; ramp.len()];

    let ok = argyll_sys::transform_u16(
        icc_data,
        argyll_sys::SRGB_ICC,
        argyll_intent,
        ramp,
        &mut out,
        npix,
    );
    if ok { Some(out) } else { None }
}

// ── Per-profile result ──────────────────────────────────────────────────

#[derive(Default)]
struct ProfileResult {
    filename: String,
    #[allow(dead_code)]
    color_space: String,
    moxcms_default_perc: Option<Vec<u16>>,
    moxcms_default_relcol: Option<Vec<u16>>,
    moxcms_tetra_perc: Option<Vec<u16>>,
    moxcms_tetra_relcol: Option<Vec<u16>>,
    lcms2_perc: Option<Vec<u16>>,
    lcms2_relcol: Option<Vec<u16>>,
    skcms_perc: Option<Vec<u16>>,
    skcms_relcol: Option<Vec<u16>>,
    argyll_perc: Option<Vec<u16>>,
    argyll_relcol: Option<Vec<u16>>,
}

// ── Report row ──────────────────────────────────────────────────────────

struct ReportRow {
    filename: String,
    intent: Intent,
    // max u16 diffs (existing pairs)
    mox_def_vs_lcms2: Option<u32>,
    mox_tet_vs_lcms2: Option<u32>,
    mox_def_vs_skcms: Option<u32>,
    mox_tet_vs_skcms: Option<u32>,
    lcms2_vs_skcms: Option<u32>,
    mox_def_vs_tet: Option<u32>,
    // ArgyllCMS pairs
    argyll_vs_lcms2: Option<u32>,
    argyll_vs_skcms: Option<u32>,
    argyll_vs_mox_def: Option<u32>,
    argyll_vs_mox_tet: Option<u32>,
    // mean diffs (existing)
    mox_def_vs_lcms2_mean: Option<f64>,
    mox_tet_vs_lcms2_mean: Option<f64>,
    mox_def_vs_skcms_mean: Option<f64>,
    mox_tet_vs_skcms_mean: Option<f64>,
    lcms2_vs_skcms_mean: Option<f64>,
    mox_def_vs_tet_mean: Option<f64>,
    // ArgyllCMS means
    argyll_vs_lcms2_mean: Option<f64>,
    argyll_vs_skcms_mean: Option<f64>,
    argyll_vs_mox_def_mean: Option<f64>,
    argyll_vs_mox_tet_mean: Option<f64>,
}

fn diff_pair(a: &Option<Vec<u16>>, b: &Option<Vec<u16>>) -> (Option<u32>, Option<f64>) {
    match (a, b) {
        (Some(a), Some(b)) => (Some(max_channel_diff(a, b)), Some(mean_channel_diff(a, b))),
        _ => (None, None),
    }
}

fn compute_row(pr: &ProfileResult, intent: Intent) -> ReportRow {
    let (mox_def, mox_tet, lcms2, skcms, argyll) = match intent {
        Intent::Perceptual => (
            &pr.moxcms_default_perc,
            &pr.moxcms_tetra_perc,
            &pr.lcms2_perc,
            &pr.skcms_perc,
            &pr.argyll_perc,
        ),
        Intent::RelativeColorimetric => (
            &pr.moxcms_default_relcol,
            &pr.moxcms_tetra_relcol,
            &pr.lcms2_relcol,
            &pr.skcms_relcol,
            &pr.argyll_relcol,
        ),
    };

    let (dl, dlm) = diff_pair(mox_def, lcms2);
    let (tl, tlm) = diff_pair(mox_tet, lcms2);
    let (ds, dsm) = diff_pair(mox_def, skcms);
    let (ts, tsm) = diff_pair(mox_tet, skcms);
    let (ls, lsm) = diff_pair(lcms2, skcms);
    let (dt, dtm) = diff_pair(mox_def, mox_tet);
    // ArgyllCMS pairs
    let (al, alm) = diff_pair(argyll, lcms2);
    let (as_, asm) = diff_pair(argyll, skcms);
    let (amd, amdm) = diff_pair(argyll, mox_def);
    let (amt, amtm) = diff_pair(argyll, mox_tet);

    ReportRow {
        filename: pr.filename.clone(),
        intent,
        mox_def_vs_lcms2: dl,
        mox_tet_vs_lcms2: tl,
        mox_def_vs_skcms: ds,
        mox_tet_vs_skcms: ts,
        lcms2_vs_skcms: ls,
        mox_def_vs_tet: dt,
        argyll_vs_lcms2: al,
        argyll_vs_skcms: as_,
        argyll_vs_mox_def: amd,
        argyll_vs_mox_tet: amt,
        mox_def_vs_lcms2_mean: dlm,
        mox_tet_vs_lcms2_mean: tlm,
        mox_def_vs_skcms_mean: dsm,
        mox_tet_vs_skcms_mean: tsm,
        lcms2_vs_skcms_mean: lsm,
        mox_def_vs_tet_mean: dtm,
        argyll_vs_lcms2_mean: alm,
        argyll_vs_skcms_mean: asm,
        argyll_vs_mox_def_mean: amdm,
        argyll_vs_mox_tet_mean: amtm,
    }
}

fn fmt_opt_u32(v: Option<u32>) -> String {
    match v {
        Some(v) => format!("{v}"),
        None => "N/A".into(),
    }
}

fn fmt_opt_f64(v: Option<f64>) -> String {
    match v {
        Some(v) => format!("{v:.1}"),
        None => "N/A".into(),
    }
}

fn print_histogram(name: &str, vals: &[u32]) {
    if vals.is_empty() {
        eprintln!("  {name:<24} no data");
        return;
    }
    let n = vals.len();
    let exact = vals.iter().filter(|&&v| v == 0).count();
    let le1 = vals.iter().filter(|&&v| v <= 1).count();
    let le2 = vals.iter().filter(|&&v| v <= 2).count();
    let le4 = vals.iter().filter(|&&v| v <= 4).count();
    let le16 = vals.iter().filter(|&&v| v <= 16).count();
    let le64 = vals.iter().filter(|&&v| v <= 64).count();
    let le256 = vals.iter().filter(|&&v| v <= 256).count();
    let le1024 = vals.iter().filter(|&&v| v <= 1024).count();
    let max = *vals.iter().max().unwrap();
    let mean: f64 = vals.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let mut sorted = vals.to_vec();
    sorted.sort();
    let median = sorted[sorted.len() / 2];
    let p95 = sorted[sorted.len() * 95 / 100];
    let p99 = sorted[sorted.len() * 99 / 100];

    eprintln!(
        "  {name:<24} n={n:>4}  exact={exact:>4}  \u{2264}1={le1:>4}  \u{2264}2={le2:>4}  \u{2264}4={le4:>4}  \u{2264}16={le16:>4}  \u{2264}64={le64:>4}  \u{2264}256={le256:>4}  \u{2264}1024={le1024:>4}  max={max:>5}  mean={mean:>7.1}  med={median:>5}  p95={p95:>5}  p99={p99:>5}"
    );
}

fn print_worst(name: &str, rows: &[(&str, u32)]) {
    if rows.is_empty() {
        return;
    }
    eprintln!("    {name}:");
    for (fname, diff) in rows.iter().take(5) {
        eprintln!("      {diff:>5} u16  {fname}");
    }
}

// ── Main test ───────────────────────────────────────────────────────────

#[test]
fn brute_force_corpus_parity() {
    let profiles = collect_all_profiles();
    let ramp = make_ramp_u16();
    let npix = num_pixels(&ramp);

    eprintln!("Brute-force CMS parity comparison");
    eprintln!("  Profiles found: {}", profiles.len());
    eprintln!("  Test ramp: {} pixels ({} u16 values)", npix, ramp.len());
    eprintln!("  Engines: moxcms (default+tetrahedral), lcms2, skcms, ArgyllCMS");
    eprintln!("  Intents: perceptual, relative colorimetric");
    eprintln!();

    let mut results: Vec<ProfileResult> = Vec::new();
    let mut parse_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut transform_fail_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut skipped_non_rgb = 0usize;

    for (idx, path) in profiles.iter().enumerate() {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        if data.len() < 132 {
            continue;
        }

        let fname = path.file_name().unwrap().to_string_lossy().to_string();

        // Skip obviously malformed test profiles
        if fname.contains("bad") || fname.contains("toosmall") || fname.contains("fuzz") {
            continue;
        }

        // Determine color space from the ICC header
        let cs_bytes = &data[16..20];
        let cs = match cs_bytes {
            b"RGB " => "RGB",
            b"GRAY" => "GRAY",
            b"CMYK" => "CMYK",
            _ => "OTHER",
        };

        if cs != "RGB" {
            skipped_non_rgb += 1;
            continue;
        }

        // Check parsability with each CMS
        let moxcms_ok = moxcms::ColorProfile::new_from_slice(&data).is_ok();
        let lcms2_ok = lcms2::Profile::new_icc(&data).is_ok();
        let skcms_ok = skcms_sys::parse_icc_profile(&data).is_some();
        // ArgyllCMS parsability is checked implicitly by the transform

        *parse_counts.entry("moxcms").or_insert(0) += moxcms_ok as usize;
        *parse_counts.entry("lcms2").or_insert(0) += lcms2_ok as usize;
        *parse_counts.entry("skcms").or_insert(0) += skcms_ok as usize;

        if !moxcms_ok && !lcms2_ok && !skcms_ok {
            continue;
        }

        if idx % 100 == 0 {
            eprintln!("  [{}/{}] {}", idx + 1, profiles.len(), fname);
        }

        let mut pr = ProfileResult {
            filename: fname.clone(),
            color_space: cs.into(),
            ..Default::default()
        };

        // Run all transforms
        pr.moxcms_default_perc = moxcms_transform_u16(&data, &ramp, Intent::Perceptual, Interp::Default);
        pr.moxcms_default_relcol = moxcms_transform_u16(&data, &ramp, Intent::RelativeColorimetric, Interp::Default);
        pr.moxcms_tetra_perc = moxcms_transform_u16(&data, &ramp, Intent::Perceptual, Interp::Tetrahedral);
        pr.moxcms_tetra_relcol = moxcms_transform_u16(&data, &ramp, Intent::RelativeColorimetric, Interp::Tetrahedral);
        pr.lcms2_perc = lcms2_transform_u16(&data, &ramp, Intent::Perceptual);
        pr.lcms2_relcol = lcms2_transform_u16(&data, &ramp, Intent::RelativeColorimetric);
        pr.skcms_perc = skcms_transform_u16(&data, &ramp, Intent::Perceptual);
        pr.skcms_relcol = skcms_transform_u16(&data, &ramp, Intent::RelativeColorimetric);
        pr.argyll_perc = argyll_transform_u16(&data, &ramp, Intent::Perceptual);
        pr.argyll_relcol = argyll_transform_u16(&data, &ramp, Intent::RelativeColorimetric);

        // Track transform failures
        if moxcms_ok && pr.moxcms_default_perc.is_none() {
            *transform_fail_counts.entry("moxcms_perc".into()).or_default() += 1;
        }
        if lcms2_ok && pr.lcms2_perc.is_none() {
            *transform_fail_counts.entry("lcms2_perc".into()).or_default() += 1;
        }
        if skcms_ok && pr.skcms_perc.is_none() {
            *transform_fail_counts.entry("skcms_perc".into()).or_default() += 1;
        }
        if pr.argyll_perc.is_none() {
            *transform_fail_counts.entry("argyll_perc".into()).or_default() += 1;
        }

        // Count argyll parse successes (any transform worked = parse succeeded)
        if pr.argyll_perc.is_some() || pr.argyll_relcol.is_some() {
            *parse_counts.entry("argyll").or_insert(0) += 1;
        }

        results.push(pr);
    }

    eprintln!("\n── Parse summary ──");
    for (cms, count) in &parse_counts {
        eprintln!("  {cms}: {count} RGB profiles parsed/transformed");
    }
    eprintln!("  Skipped non-RGB: {skipped_non_rgb}");

    if !transform_fail_counts.is_empty() {
        eprintln!("\n── Transform failures ──");
        for (k, v) in &transform_fail_counts {
            eprintln!("  {k}: {v}");
        }
    }

    // ── Build report rows ────────────────────────────────────────────

    let mut rows: Vec<ReportRow> = Vec::new();
    for pr in &results {
        rows.push(compute_row(pr, Intent::Perceptual));
        rows.push(compute_row(pr, Intent::RelativeColorimetric));
    }

    // ── Write TSV ────────────────────────────────────────────────────

    let out_dir = PathBuf::from("/mnt/v/output/oxcms");
    std::fs::create_dir_all(&out_dir).ok();
    let tsv_path = out_dir.join("brute_force_parity.tsv");
    let mut f = std::io::BufWriter::new(std::fs::File::create(&tsv_path).expect("create TSV"));

    writeln!(
        f,
        "profile\tintent\t\
         mox_def_vs_lcms2\tmox_tet_vs_lcms2\tmox_def_vs_skcms\tmox_tet_vs_skcms\t\
         lcms2_vs_skcms\tmox_def_vs_tet\t\
         argyll_vs_lcms2\targyll_vs_skcms\targyll_vs_mox_def\targyll_vs_mox_tet\t\
         mox_def_vs_lcms2_mean\tmox_tet_vs_lcms2_mean\tmox_def_vs_skcms_mean\tmox_tet_vs_skcms_mean\t\
         lcms2_vs_skcms_mean\tmox_def_vs_tet_mean\t\
         argyll_vs_lcms2_mean\targyll_vs_skcms_mean\targyll_vs_mox_def_mean\targyll_vs_mox_tet_mean"
    ).unwrap();

    for row in &rows {
        writeln!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.filename,
            row.intent,
            fmt_opt_u32(row.mox_def_vs_lcms2),
            fmt_opt_u32(row.mox_tet_vs_lcms2),
            fmt_opt_u32(row.mox_def_vs_skcms),
            fmt_opt_u32(row.mox_tet_vs_skcms),
            fmt_opt_u32(row.lcms2_vs_skcms),
            fmt_opt_u32(row.mox_def_vs_tet),
            fmt_opt_u32(row.argyll_vs_lcms2),
            fmt_opt_u32(row.argyll_vs_skcms),
            fmt_opt_u32(row.argyll_vs_mox_def),
            fmt_opt_u32(row.argyll_vs_mox_tet),
            fmt_opt_f64(row.mox_def_vs_lcms2_mean),
            fmt_opt_f64(row.mox_tet_vs_lcms2_mean),
            fmt_opt_f64(row.mox_def_vs_skcms_mean),
            fmt_opt_f64(row.mox_tet_vs_skcms_mean),
            fmt_opt_f64(row.lcms2_vs_skcms_mean),
            fmt_opt_f64(row.mox_def_vs_tet_mean),
            fmt_opt_f64(row.argyll_vs_lcms2_mean),
            fmt_opt_f64(row.argyll_vs_skcms_mean),
            fmt_opt_f64(row.argyll_vs_mox_def_mean),
            fmt_opt_f64(row.argyll_vs_mox_tet_mean),
        ).unwrap();
    }
    drop(f);
    eprintln!("\nTSV written to {}", tsv_path.display());

    // ── Console summary statistics ───────────────────────────────────

    for intent in [Intent::Perceptual, Intent::RelativeColorimetric] {
        let intent_rows: Vec<&ReportRow> = rows.iter().filter(|r| r.intent == intent).collect();

        eprintln!("\n══ Intent: {intent} ({} profiles) ══", intent_rows.len());

        let pairs: Vec<(&str, Box<dyn Fn(&&ReportRow) -> Option<u32>>)> = vec![
            ("moxcms_def vs lcms2", Box::new(|r: &&ReportRow| r.mox_def_vs_lcms2)),
            ("moxcms_tet vs lcms2", Box::new(|r: &&ReportRow| r.mox_tet_vs_lcms2)),
            ("moxcms_def vs skcms", Box::new(|r: &&ReportRow| r.mox_def_vs_skcms)),
            ("moxcms_tet vs skcms", Box::new(|r: &&ReportRow| r.mox_tet_vs_skcms)),
            ("lcms2 vs skcms",      Box::new(|r: &&ReportRow| r.lcms2_vs_skcms)),
            ("moxcms_def vs tet",   Box::new(|r: &&ReportRow| r.mox_def_vs_tet)),
            ("argyll vs lcms2",     Box::new(|r: &&ReportRow| r.argyll_vs_lcms2)),
            ("argyll vs skcms",     Box::new(|r: &&ReportRow| r.argyll_vs_skcms)),
            ("argyll vs moxcms_def",Box::new(|r: &&ReportRow| r.argyll_vs_mox_def)),
            ("argyll vs moxcms_tet",Box::new(|r: &&ReportRow| r.argyll_vs_mox_tet)),
        ];

        for (name, accessor) in &pairs {
            let vals: Vec<u32> = intent_rows.iter().filter_map(|r| accessor(r)).collect();
            print_histogram(name, &vals);
        }

        eprintln!("\n  Top 5 worst per comparison:");
        for (name, accessor) in &pairs {
            let mut worst: Vec<(&str, u32)> = intent_rows
                .iter()
                .filter_map(|r| accessor(r).map(|v| (r.filename.as_str(), v)))
                .collect();
            worst.sort_by(|a, b| b.1.cmp(&a.1));
            worst.truncate(5);
            print_worst(name, &worst);
        }
    }

    // ── Profiles where intents diverge significantly ─────────────────

    eprintln!("\n══ Profiles where Perceptual ≠ RelCol (max > 256 u16) ══");
    let mut intent_divergent = 0;
    for pr in &results {
        for (name, a, b) in [
            ("moxcms_def", &pr.moxcms_default_perc, &pr.moxcms_default_relcol),
            ("moxcms_tet", &pr.moxcms_tetra_perc, &pr.moxcms_tetra_relcol),
            ("lcms2", &pr.lcms2_perc, &pr.lcms2_relcol),
            ("skcms", &pr.skcms_perc, &pr.skcms_relcol),
            ("argyll", &pr.argyll_perc, &pr.argyll_relcol),
        ] {
            let (max, _) = diff_pair(a, b);
            if let Some(d) = max {
                if d > 256 {
                    if intent_divergent < 50 {
                        eprintln!("  {:<12} {:>5} u16  {}", name, d, pr.filename);
                    }
                    intent_divergent += 1;
                }
            }
        }
    }
    if intent_divergent > 50 {
        eprintln!("  ... and {} more", intent_divergent - 50);
    }
    eprintln!("  Total intent-divergent entries: {intent_divergent}");

    eprintln!("\nDone. Full results in {}", tsv_path.display());
}
