//! Performance benchmarks for CMS transform operations
//!
//! Compares moxcms, lcms2, qcms, and skcms transform performance.
//! Tests both profile parsing and pixel transforms at various sizes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use skcms_sys::{skcms_AlphaFormat, skcms_PixelFormat};

const PIXEL_COUNTS: &[usize] = &[1, 16, 256, 4096, 65536, 262144];

fn bench_srgb_identity_8bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("sRGB Identity 8-bit");

    for &count in PIXEL_COUNTS {
        let size = count * 3;
        group.throughput(Throughput::Bytes(size as u64));

        // Generate test data
        let input: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let mut output = vec![0u8; size];

        // Setup moxcms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = moxcms_srgb
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &moxcms_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        group.bench_with_input(BenchmarkId::new("moxcms", count), &count, |b, _| {
            b.iter(|| {
                moxcms_transform
                    .transform(black_box(&input), black_box(&mut output))
                    .unwrap()
            })
        });

        // Setup lcms2
        let lcms2_srgb = lcms2::Profile::new_srgb();
        let lcms2_transform = lcms2::Transform::new(
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_8,
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_8,
            lcms2::Intent::Perceptual,
        )
        .unwrap();

        group.bench_with_input(BenchmarkId::new("lcms2", count), &count, |b, _| {
            b.iter(|| {
                lcms2_transform.transform_pixels(black_box(&input), black_box(&mut output))
            })
        });

        // Setup qcms (in-place only)
        let qcms_srgb = qcms::Profile::new_sRGB();
        let qcms_transform = qcms::Transform::new(
            &qcms_srgb,
            &qcms_srgb,
            qcms::DataType::RGB8,
            qcms::Intent::Perceptual,
        )
        .unwrap();

        group.bench_with_input(BenchmarkId::new("qcms", count), &count, |b, _| {
            b.iter(|| {
                let mut data = input.clone();
                qcms_transform.apply(black_box(&mut data))
            })
        });

        // Setup skcms
        let skcms_srgb = skcms_sys::srgb_profile();

        group.bench_with_input(BenchmarkId::new("skcms", count), &count, |b, _| {
            b.iter(|| {
                skcms_sys::transform(
                    black_box(&input),
                    skcms_PixelFormat::RGB_888,
                    skcms_AlphaFormat::Opaque,
                    skcms_srgb,
                    black_box(&mut output),
                    skcms_PixelFormat::RGB_888,
                    skcms_AlphaFormat::Opaque,
                    skcms_srgb,
                    count,
                )
            })
        });
    }

    group.finish();
}

fn bench_srgb_to_p3_8bit(c: &mut Criterion) {
    let mut group = c.benchmark_group("sRGB to Display P3 8-bit");

    for &count in PIXEL_COUNTS {
        let size = count * 3;
        group.throughput(Throughput::Bytes(size as u64));

        // Generate test data
        let input: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let mut output = vec![0u8; size];

        // Setup moxcms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_p3 = moxcms::ColorProfile::new_display_p3();
        let moxcms_transform = moxcms_srgb
            .create_transform_8bit(
                moxcms::Layout::Rgb,
                &moxcms_p3,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        group.bench_with_input(BenchmarkId::new("moxcms", count), &count, |b, _| {
            b.iter(|| {
                moxcms_transform
                    .transform(black_box(&input), black_box(&mut output))
                    .unwrap()
            })
        });

        // Note: lcms2, qcms, and skcms don't have built-in P3 profiles
        // This benchmark focuses on comparing transform performance for matrix-shaper profiles
    }

    group.finish();
}

fn bench_profile_parsing(c: &mut Criterion) {
    use std::path::Path;

    let testdata = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata");

    // Collect some test profiles
    let profiles = [
        testdata.join("profiles/skcms/misc/AdobeRGB.icc"),
        testdata.join("profiles/qcms/sRGB_lcms.icc"),
        testdata.join("profiles/skcms/mobile/Display_P3_parametric.icc"),
    ];

    let mut group = c.benchmark_group("Profile Parsing");

    for profile_path in &profiles {
        if !profile_path.exists() {
            continue;
        }

        let data = std::fs::read(profile_path).unwrap();
        let name = profile_path.file_name().unwrap().to_string_lossy();
        group.throughput(Throughput::Bytes(data.len() as u64));

        group.bench_with_input(BenchmarkId::new("moxcms", &name), &data, |b, data| {
            b.iter(|| moxcms::ColorProfile::new_from_slice(black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("lcms2", &name), &data, |b, data| {
            b.iter(|| lcms2::Profile::new_icc(black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("qcms", &name), &data, |b, data| {
            b.iter(|| qcms::Profile::new_from_slice(black_box(data), false))
        });

        group.bench_with_input(BenchmarkId::new("skcms", &name), &data, |b, data| {
            b.iter(|| skcms_sys::parse_icc_profile(black_box(data)))
        });
    }

    group.finish();
}

fn bench_rgba_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("RGBA Transform with Alpha");

    for &count in &[1, 256, 4096, 65536] {
        let size = count * 4; // RGBA
        group.throughput(Throughput::Bytes(size as u64));

        // Generate test data with alpha
        let input: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let mut output = vec![0u8; size];

        // Setup moxcms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = moxcms_srgb
            .create_transform_8bit(
                moxcms::Layout::Rgba,
                &moxcms_srgb,
                moxcms::Layout::Rgba,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        group.bench_with_input(BenchmarkId::new("moxcms", count), &count, |b, _| {
            b.iter(|| {
                moxcms_transform
                    .transform(black_box(&input), black_box(&mut output))
                    .unwrap()
            })
        });

        // Setup lcms2
        let lcms2_srgb = lcms2::Profile::new_srgb();
        let lcms2_transform = lcms2::Transform::new(
            &lcms2_srgb,
            lcms2::PixelFormat::RGBA_8,
            &lcms2_srgb,
            lcms2::PixelFormat::RGBA_8,
            lcms2::Intent::Perceptual,
        )
        .unwrap();

        group.bench_with_input(BenchmarkId::new("lcms2", count), &count, |b, _| {
            b.iter(|| {
                lcms2_transform.transform_pixels(black_box(&input), black_box(&mut output))
            })
        });

        // Setup skcms
        let skcms_srgb = skcms_sys::srgb_profile();

        group.bench_with_input(BenchmarkId::new("skcms", count), &count, |b, _| {
            b.iter(|| {
                skcms_sys::transform(
                    black_box(&input),
                    skcms_PixelFormat::RGBA_8888,
                    skcms_AlphaFormat::Unpremul,
                    skcms_srgb,
                    black_box(&mut output),
                    skcms_PixelFormat::RGBA_8888,
                    skcms_AlphaFormat::Unpremul,
                    skcms_srgb,
                    count,
                )
            })
        });
    }

    group.finish();
}

fn bench_16bit_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("16-bit Transforms");

    for &count in &[1, 256, 4096, 65536] {
        let size = count * 3; // RGB
        group.throughput(Throughput::Bytes((size * 2) as u64)); // u16 = 2 bytes

        // Generate test data
        let input: Vec<u16> = (0..size).map(|i| ((i * 257) % 65536) as u16).collect();
        let mut output = vec![0u16; size];

        // Setup moxcms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = moxcms_srgb
            .create_transform_16bit(
                moxcms::Layout::Rgb,
                &moxcms_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        group.bench_with_input(BenchmarkId::new("moxcms", count), &count, |b, _| {
            b.iter(|| {
                moxcms_transform
                    .transform(black_box(&input), black_box(&mut output))
                    .unwrap()
            })
        });

        // Setup lcms2 (uses u16 arrays)
        let lcms2_srgb = lcms2::Profile::new_srgb();
        let lcms2_transform = lcms2::Transform::new(
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_16,
            &lcms2_srgb,
            lcms2::PixelFormat::RGB_16,
            lcms2::Intent::Perceptual,
        )
        .unwrap();

        // lcms2 transform_pixels takes &[u8], need to cast
        let input_u8: Vec<u8> = input
            .iter()
            .flat_map(|&v| v.to_ne_bytes())
            .collect();
        let mut output_u8 = vec![0u8; size * 2];

        group.bench_with_input(BenchmarkId::new("lcms2", count), &count, |b, _| {
            b.iter(|| {
                lcms2_transform.transform_pixels(black_box(&input_u8), black_box(&mut output_u8))
            })
        });

        // skcms supports 16-bit (uses native endian via LE on x86)
        let skcms_srgb = skcms_sys::srgb_profile();

        group.bench_with_input(BenchmarkId::new("skcms", count), &count, |b, _| {
            b.iter(|| {
                skcms_sys::transform_u16(
                    black_box(&input),
                    skcms_PixelFormat::RGB_161616LE,
                    skcms_AlphaFormat::Opaque,
                    skcms_srgb,
                    black_box(&mut output),
                    skcms_PixelFormat::RGB_161616LE,
                    skcms_AlphaFormat::Opaque,
                    skcms_srgb,
                    count,
                )
            })
        });
    }

    group.finish();
}

fn bench_f32_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("F32 Transforms");

    for &count in &[1, 256, 4096, 65536] {
        let size = count * 3; // RGB
        group.throughput(Throughput::Bytes((size * 4) as u64)); // f32 = 4 bytes

        // Generate test data
        let input: Vec<f32> = (0..size).map(|i| i as f32 / size as f32).collect();
        let mut output = vec![0f32; size];

        // Setup moxcms
        let moxcms_srgb = moxcms::ColorProfile::new_srgb();
        let moxcms_transform = moxcms_srgb
            .create_transform_f32(
                moxcms::Layout::Rgb,
                &moxcms_srgb,
                moxcms::Layout::Rgb,
                moxcms::TransformOptions::default(),
            )
            .unwrap();

        group.bench_with_input(BenchmarkId::new("moxcms", count), &count, |b, _| {
            b.iter(|| {
                moxcms_transform
                    .transform(black_box(&input), black_box(&mut output))
                    .unwrap()
            })
        });

        // skcms supports f32
        let skcms_srgb = skcms_sys::srgb_profile();

        group.bench_with_input(BenchmarkId::new("skcms", count), &count, |b, _| {
            b.iter(|| {
                skcms_sys::transform_f32(
                    black_box(&input),
                    skcms_PixelFormat::RGB_fff,
                    skcms_AlphaFormat::Opaque,
                    skcms_srgb,
                    black_box(&mut output),
                    skcms_PixelFormat::RGB_fff,
                    skcms_AlphaFormat::Opaque,
                    skcms_srgb,
                    count,
                )
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_srgb_identity_8bit,
    bench_srgb_to_p3_8bit,
    bench_profile_parsing,
    bench_rgba_transforms,
    bench_16bit_transforms,
    bench_f32_transforms,
);

criterion_main!(benches);
