//! Behavioral witnesses, not a general ICC conformance certification.
//! See docs/ICC_DEVIATIONS.md for the lcms2.20 comparison and spec clauses.
#![forbid(unsafe_code)]

use moxcms::{ColorProfile, ToneReprCurve};

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

fn tag<'a>(bytes: &'a [u8], sig: &[u8; 4]) -> Option<&'a [u8]> {
    (0..u32_at(bytes, 128)).find_map(|i| {
        let at = 132 + i * 12;
        (&bytes[at..at + 4] == sig).then(|| {
            let start = u32_at(bytes, at + 4);
            &bytes[start..start + u32_at(bytes, at + 8)]
        })
    })
}

// Rebuild the table so generated tags do not need committed binary fixtures.
fn with_tag(base: &[u8], sig: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut tags: Vec<([u8; 4], Vec<u8>)> = (0..u32_at(base, 128))
        .filter_map(|i| {
            let at = 132 + i * 12;
            let key: [u8; 4] = base[at..at + 4].try_into().unwrap();
            (key != *sig).then(|| (key, tag(base, &key).unwrap().to_vec()))
        })
        .collect();
    tags.push((*sig, data.to_vec()));
    let mut out = base[..128].to_vec();
    out.extend_from_slice(&(tags.len() as u32).to_be_bytes());
    out.resize(132 + tags.len() * 12, 0);
    for (i, (key, value)) in tags.iter().enumerate() {
        let at = 132 + i * 12;
        let offset = out.len() as u32;
        out[at..at + 4].copy_from_slice(key);
        out[at + 4..at + 8].copy_from_slice(&offset.to_be_bytes());
        out[at + 8..at + 12].copy_from_slice(&(value.len() as u32).to_be_bytes());
        out.extend_from_slice(value);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    let len = out.len() as u32;
    out[..4].copy_from_slice(&len.to_be_bytes());
    out
}

#[test]
fn d3_gray_uses_d50_and_records_adaptation() {
    let p = ColorProfile::new_gray_with_gamma(2.2);
    let bytes = p.encode().unwrap();
    let wtpt = tag(&bytes, b"wtpt").unwrap();
    let xyz: Vec<f64> = (8..20)
        .step_by(4)
        .map(|i| i32::from_be_bytes(wtpt[i..i + 4].try_into().unwrap()) as f64 / 65536.0)
        .collect();
    println!(
        "D3 gray wtpt={xyz:?}; chad={}",
        tag(&bytes, b"chad").is_some()
    );
    for (got, expected) in xyz.iter().zip([0.9642, 1.0, 0.8249]) {
        assert!((got - expected).abs() < 2.0 / 65536.0);
    }
    assert!(tag(&bytes, b"chad").is_some());
}

#[test]
fn d6_d9_d10_writer_upgrades_v2_and_zeros_id() {
    let mut bytes = ColorProfile::new_srgb().encode().unwrap();
    bytes[8..12].copy_from_slice(&[2, 0x10, 0, 0]);
    bytes[84..100].fill(0x5a);
    let encoded = ColorProfile::new_from_slice(&bytes)
        .unwrap()
        .encode()
        .unwrap();
    println!(
        "D6/D9 input v2 -> output version={:?}; D10 ID={:?}",
        &encoded[8..12],
        &encoded[84..100]
    );
    assert_eq!(encoded[8], 4);
    assert_eq!(&encoded[84..100], &[0; 16]);
}

#[test]
fn d4_d8_legacy_description_becomes_english_mluc() {
    let mut desc = b"desc\0\0\0\0".to_vec();
    desc.extend_from_slice(&2u32.to_be_bytes()); // ASCII byte count
    desc.extend_from_slice(b"a\0");
    desc.extend_from_slice(&[0; 8]); // no Unicode language or string
    desc.extend_from_slice(&[0; 70]); // empty ScriptCode data
    let mut input = with_tag(&ColorProfile::new_srgb().encode().unwrap(), b"desc", &desc);
    input[8..12].copy_from_slice(&[2, 0x10, 0, 0]);
    let output = ColorProfile::new_from_slice(&input)
        .unwrap()
        .encode()
        .unwrap();
    let description = tag(&output, b"desc").unwrap();
    assert_eq!(&description[..4], b"mluc");
    assert_eq!(&description[16..20], b"enUS");
    assert_eq!(description.len(), 30); // no padding included in tag size
    println!("D4/D8 v2 desc -> v4 mluc enUS, 30-byte tag excluding alignment padding");
}

#[test]
fn d5_d11_d12_unsupported_metadata_is_dropped() {
    // Valid empty instances: their round-trip loss is separate from the
    // alignment and embedded-mluc issues found in lcms's nonempty writers.
    let mut bytes = ColorProfile::new_srgb().encode().unwrap();
    bytes = with_tag(&bytes, b"psid", b"psid\0\0\0\0\0\0\0\0");
    bytes = with_tag(&bytes, b"meta", b"dict\0\0\0\0\0\0\0\0\0\0\0\x10");
    let encoded = ColorProfile::new_from_slice(&bytes)
        .unwrap()
        .encode()
        .unwrap();
    assert!(tag(&encoded, b"psid").is_none());
    assert!(tag(&encoded, b"meta").is_none());
    println!("D5/D11/D12 psid and dict are dropped, not conformantly round-tripped");
}

#[test]
fn d13_direct_curve_evaluation_is_unbounded() {
    for params in [vec![1.0, 2.0, 0.0], vec![1.0, 1.0, 0.0, -0.25]] {
        let curve = ToneReprCurve::Parametric(params.clone());
        let eval = curve.make_linear_evaluator().unwrap();
        let x = if params.len() == 3 { 0.75 } else { 0.0 };
        let got = eval.evaluate_value(x);
        println!("D13 params={params:?}, x={x}, result={got}");
        assert_eq!(got, if params.len() == 3 { 1.5 } else { -0.25 });
    }
}

#[test]
fn d13_float_transform_depends_on_extended_range() {
    use moxcms::{Layout, TransformOptions};
    let mut source = ColorProfile::new_srgb();
    source.cicp = None;
    source.red_trc = Some(ToneReprCurve::Parametric(vec![1.0, 2.0, 0.0]));
    source.green_trc = source.red_trc.clone();
    source.blue_trc = source.red_trc.clone();
    let mut dest = source.clone();
    dest.red_trc = Some(ToneReprCurve::Parametric(vec![1.0]));
    dest.green_trc = dest.red_trc.clone();
    dest.blue_trc = dest.red_trc.clone();
    let transform = source
        .create_transform_f32(Layout::Rgb, &dest, Layout::Rgb, TransformOptions::default())
        .unwrap();
    let mut out = [0.0f32; 3];
    transform.transform(&[0.75; 3], &mut out).unwrap();
    println!(
        "D13 RGB f32 defaults, extended_range compiled={} -> {out:?}",
        cfg!(feature = "extended-range")
    );
    for value in out {
        assert!((value - 1.0).abs() < 0.001);
    }
    #[cfg(feature = "extended-range")]
    {
        let options = TransformOptions {
            allow_extended_range_rgb_xyz: true,
            ..Default::default()
        };
        let transform = source
            .create_transform_f32(Layout::Rgb, &dest, Layout::Rgb, options)
            .unwrap();
        transform.transform(&[0.75; 3], &mut out).unwrap();
        println!("D13 RGB f32 with extended range enabled -> {out:?}");
        for value in out {
            assert!((value - 1.5).abs() < 0.001);
        }
    }
}

#[test]
fn d7_writer_emits_a_supplied_zero_grid_lut16() {
    use moxcms::{LutDataType, LutStore, LutType, LutWarehouse, Matrix3d};
    let mut profile = ColorProfile::new_srgb();
    profile.lut_a_to_b_perceptual = Some(LutWarehouse::Lut(LutDataType {
        num_input_channels: 3,
        num_output_channels: 3,
        num_clut_grid_points: 0,
        matrix: Matrix3d::IDENTITY,
        num_input_table_entries: 2,
        num_output_table_entries: 2,
        input_table: LutStore::Store16(vec![0, 65535, 0, 65535, 0, 65535]),
        clut_table: LutStore::Store16(vec![]),
        output_table: LutStore::Store16(vec![0, 65535, 0, 65535, 0, 65535]),
        lut_type: LutType::Lut16,
    }));
    let bytes = profile.encode().unwrap();
    let lut = tag(&bytes, b"A2B0").unwrap();
    assert_eq!(&lut[..4], b"mft2");
    assert_eq!(lut[10], 0);
    println!(
        "D7 supplied zero-grid lut16 is emitted; parser accepts={}",
        ColorProfile::new_from_slice(&bytes).is_ok()
    );
}

#[test]
fn d14_inverse_plateaus_at_decimal_and_stored_values() {
    for n in [4096, 65536] {
        for (name, plateau) in [("mid", 0.5f64), ("end", 0.8), ("end_one", 1.0)] {
            let values: Vec<u16> = (0..n)
                .map(|i| {
                    let x = i as f64 / (n - 1) as f64;
                    let y = if name == "mid" {
                        if x < 0.3 {
                            x / 0.3 * plateau
                        } else if x <= 0.6 {
                            plateau
                        } else {
                            plateau + (x - 0.6) / 0.4 * (1.0 - plateau)
                        }
                    } else if x < 0.5 {
                        x / 0.5 * plateau
                    } else {
                        plateau
                    };
                    (y * 65535.0).round() as u16
                })
                .collect();
            let forward = ToneReprCurve::Lut(values);
            let inverse = forward.inverse().unwrap().make_linear_evaluator().unwrap();
            let gamma = forward.make_gamma_evaluator().unwrap();
            let stored = (plateau * 65535.0).round() as f32 / 65535.0;
            let decimal_result = inverse.evaluate_value(plateau as f32);
            let stored_result = inverse.evaluate_value(stored);
            let gamma_result = gamma.evaluate_value(stored);
            println!(
                "D14 n={n} {name}: decimal={decimal_result:.9}, stored={stored_result:.9}, gamma(stored)={gamma_result:.9}"
            );
            // Both inverse APIs resample a discontinuity. These are behavior
            // witnesses, not assertions that the current result follows F.1.
            if name == "end" {
                assert!((stored_result - 0.5).abs() > 0.05);
            }
            if name == "mid" && n == 65536 {
                assert!((stored_result - 0.5).abs() < 0.001);
                assert!((stored_result - 0.6).abs() > 0.05);
            }
        }
    }
}

#[test]
fn d15_extended_cicp_is_accepted_but_extension_is_lost() {
    let base = ColorProfile::new_srgb().encode().unwrap();
    let mut cicp = b"cicp\0\0\0\0\x02\x0d\0\x01".to_vec();
    for v in [0.64f64, 0.33, 0.30, 0.60, 0.15, 0.06, 0.3127, 0.3290] {
        cicp.extend_from_slice(&((v * 65536.0).round() as i32).to_be_bytes());
    }
    let input = with_tag(&base, b"cicp", &cicp);
    let profile = ColorProfile::new_from_slice(&input).unwrap();
    assert_eq!(profile.cicp.unwrap().color_primaries as u8, 2);
    let output = profile.encode().unwrap();
    assert_eq!(tag(&output, b"cicp").unwrap(), &cicp[..12]);
    println!(
        "D15 accepted 44-byte cicp; encoded {} bytes",
        tag(&output, b"cicp").unwrap().len()
    );
}

#[test]
fn native_oxcms_also_has_unbounded_parametric_output() {
    use oxcms_core::math::gamma::{ParametricCurve, ParametricCurveType, parametric_curve_eval};
    let curve =
        ParametricCurve::from_params(ParametricCurveType::CIE122, &[1.0, 2.0, 0.0]).unwrap();
    let got = parametric_curve_eval(&curve, 0.75);
    println!("oxcms native D13 x=0.75 -> {got}");
    assert_eq!(got, 1.5);
}

#[test]
fn native_oxcms_chooses_low_edge_of_plateaus() {
    use oxcms_core::icc::tags::CurveData;
    for (name, plateau) in [("mid", 0.5f64), ("end", 0.8)] {
        let table = (0..4096)
            .map(|i| {
                let x = i as f64 / 4095.0;
                let y = if name == "mid" {
                    if x < 0.3 {
                        x / 0.3 * plateau
                    } else if x <= 0.6 {
                        plateau
                    } else {
                        plateau + (x - 0.6) / 0.4 * (1.0 - plateau)
                    }
                } else if x < 0.5 {
                    x / 0.5 * plateau
                } else {
                    plateau
                };
                (y * 65535.0).round() as u16
            })
            .collect();
        let y = (plateau * 65535.0).round() / 65535.0;
        let x = CurveData::Table(table).eval_inverse(y);
        println!("oxcms native D14 {name}: stored={x:.9}");
        assert!((x - if name == "mid" { 0.3 } else { 0.5 }).abs() < 0.001);
    }
}
