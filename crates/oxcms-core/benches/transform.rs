//! SIMD Transform Benchmarks
//!
//! Benchmarks to identify which SIMD functions benefit from further optimization.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use oxcms_core::simd;

/// sRGB to XYZ matrix (D65 adapted)
const SRGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
];

/// Generate test data for benchmarks
fn generate_rgb_data(count: usize) -> Vec<[f64; 3]> {
    (0..count)
        .map(|i| {
            let t = i as f64 / count as f64;
            [t, (t * 2.0) % 1.0, (t * 3.0) % 1.0]
        })
        .collect()
}

fn generate_rgb8_data(count: usize) -> Vec<u8> {
    (0..count * 3).map(|i| ((i * 37) % 256) as u8).collect()
}

fn generate_f64_values(count: usize) -> Vec<f64> {
    (0..count).map(|i| (i as f64 / count as f64)).collect()
}

fn generate_lut(size: usize) -> Vec<f64> {
    // sRGB-like gamma curve as LUT
    (0..size)
        .map(|i| {
            let x = i as f64 / (size - 1) as f64;
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect()
}

// ============================================================================
// Matrix Multiply Benchmarks
// ============================================================================

fn bench_matrix_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_single");

    let vec = [0.5, 0.3, 0.7];

    group.bench_function("simd_matrix_multiply_vec3", |b| {
        b.iter(|| simd::matrix_multiply_vec3(black_box(&SRGB_TO_XYZ), black_box(vec)))
    });

    // Scalar baseline for comparison
    group.bench_function("scalar_matrix_multiply_vec3", |b| {
        b.iter(|| {
            let m = black_box(&SRGB_TO_XYZ);
            let v = black_box(vec);
            [
                m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
                m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
                m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
            ]
        })
    });

    group.finish();
}

fn bench_matrix_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("matrix_batch");

    for size in [100, 1000, 10000, 100000].iter() {
        let input = generate_rgb_data(*size);
        let mut output = vec![[0.0f64; 3]; *size];

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| {
                simd::matrix_multiply_vec3_batch(
                    black_box(&SRGB_TO_XYZ),
                    black_box(&input),
                    black_box(&mut output),
                )
            })
        });

        // Scalar baseline
        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| {
                let m = &SRGB_TO_XYZ;
                for (inp, out) in input.iter().zip(output.iter_mut()) {
                    out[0] = m[0][0] * inp[0] + m[0][1] * inp[1] + m[0][2] * inp[2];
                    out[1] = m[1][0] * inp[0] + m[1][1] * inp[1] + m[1][2] * inp[2];
                    out[2] = m[2][0] * inp[0] + m[2][1] * inp[1] + m[2][2] * inp[2];
                }
            })
        });
    }

    group.finish();
}

// ============================================================================
// Gamma / Transfer Function Benchmarks
// ============================================================================

fn bench_gamma(c: &mut Criterion) {
    let mut group = c.benchmark_group("gamma");

    for size in [1000, 10000, 100000].iter() {
        let input = generate_f64_values(*size);
        let mut output = vec![0.0f64; *size];

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd_gamma_2.2", size), size, |b, _| {
            b.iter(|| {
                simd::apply_gamma_batch(black_box(&input), black_box(&mut output), black_box(2.2))
            })
        });

        // Scalar baseline
        group.bench_with_input(BenchmarkId::new("scalar_gamma_2.2", size), size, |b, _| {
            b.iter(|| {
                for (inp, out) in input.iter().zip(output.iter_mut()) {
                    *out = inp.clamp(0.0, 1.0).powf(2.2);
                }
            })
        });
    }

    group.finish();
}

fn bench_srgb_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("srgb_decode");

    for size in [1000, 10000, 100000].iter() {
        let input = generate_f64_values(*size);
        let mut output = vec![0.0f64; *size];

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd::apply_srgb_decode_batch(black_box(&input), black_box(&mut output)))
        });

        // Scalar baseline
        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| {
                const THRESHOLD: f64 = 0.04045;
                const LINEAR_SCALE: f64 = 1.0 / 12.92;
                const POWER_OFFSET: f64 = 0.055;
                const POWER_SCALE: f64 = 1.0 / 1.055;

                for (inp, out) in input.iter().zip(output.iter_mut()) {
                    let x = inp.clamp(0.0, 1.0);
                    *out = if x <= THRESHOLD {
                        x * LINEAR_SCALE
                    } else {
                        ((x + POWER_OFFSET) * POWER_SCALE).powf(2.4)
                    };
                }
            })
        });
    }

    group.finish();
}

fn bench_srgb_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("srgb_encode");

    for size in [1000, 10000, 100000].iter() {
        // Use linear values for encoding
        let input: Vec<f64> = generate_f64_values(*size)
            .iter()
            .map(|x| x.powf(2.2)) // Pre-linearize
            .collect();
        let mut output = vec![0.0f64; *size];

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd::apply_srgb_encode_batch(black_box(&input), black_box(&mut output)))
        });

        // Scalar baseline
        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| {
                const THRESHOLD: f64 = 0.0031308;
                const LINEAR_SCALE: f64 = 12.92;
                const POWER_SCALE: f64 = 1.055;
                const POWER_OFFSET: f64 = 0.055;
                const POWER_EXP: f64 = 1.0 / 2.4;

                for (inp, out) in input.iter().zip(output.iter_mut()) {
                    let x = inp.clamp(0.0, 1.0);
                    *out = if x <= THRESHOLD {
                        x * LINEAR_SCALE
                    } else {
                        POWER_SCALE * x.powf(POWER_EXP) - POWER_OFFSET
                    };
                }
            })
        });
    }

    group.finish();
}

// ============================================================================
// LUT Interpolation Benchmarks
// ============================================================================

fn bench_lut1d(c: &mut Criterion) {
    let mut group = c.benchmark_group("lut1d");

    let lut = generate_lut(256);

    for size in [1000, 10000, 100000].iter() {
        let input = generate_f64_values(*size);
        let mut output = vec![0.0f64; *size];

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| {
                simd::apply_lut1d_batch(black_box(&input), black_box(&mut output), black_box(&lut))
            })
        });

        // Scalar baseline with linear interpolation
        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| {
                let lut_max = (lut.len() - 1) as f64;
                for (inp, out) in input.iter().zip(output.iter_mut()) {
                    let x = inp.clamp(0.0, 1.0);
                    let pos = x * lut_max;
                    let idx = pos.floor() as usize;
                    let frac = pos - idx as f64;

                    if idx >= lut.len() - 1 {
                        *out = lut[lut.len() - 1];
                    } else {
                        *out = lut[idx] + frac * (lut[idx + 1] - lut[idx]);
                    }
                }
            })
        });
    }

    group.finish();
}

// ============================================================================
// RGB8 Batch Transform Benchmarks
// ============================================================================

fn bench_rgb8_batch_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("rgb8_batch_transform");

    for pixel_count in [1000, 10000, 100000].iter() {
        let src = generate_rgb8_data(*pixel_count);
        let mut dst = vec![0u8; src.len()];

        group.throughput(Throughput::Elements(*pixel_count as u64));

        // Identity transform
        group.bench_with_input(
            BenchmarkId::new("identity", pixel_count),
            pixel_count,
            |b, _| {
                b.iter(|| {
                    simd::transform_rgb8_batch(black_box(&src), black_box(&mut dst), |rgb| rgb)
                })
            },
        );

        // Matrix transform (sRGB to XYZ-ish)
        group.bench_with_input(
            BenchmarkId::new("matrix_transform", pixel_count),
            pixel_count,
            |b, _| {
                b.iter(|| {
                    simd::transform_rgb8_batch(black_box(&src), black_box(&mut dst), |rgb| {
                        simd::matrix_multiply_vec3(&SRGB_TO_XYZ, rgb)
                    })
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// RGB8 <-> f64 Conversion Benchmarks
// ============================================================================

fn bench_rgb8_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("rgb8_conversion");

    for pixel_count in [1000, 10000, 100000].iter() {
        let src_u8 = generate_rgb8_data(*pixel_count);
        let mut f64_buf = vec![[0.0f64; 3]; *pixel_count];
        let mut dst_u8 = vec![0u8; src_u8.len()];

        group.throughput(Throughput::Elements(*pixel_count as u64));

        group.bench_with_input(
            BenchmarkId::new("rgb8_to_f64", pixel_count),
            pixel_count,
            |b, _| b.iter(|| simd::rgb8_to_f64_batch(black_box(&src_u8), black_box(&mut f64_buf))),
        );

        // Pre-fill f64_buf for the reverse test
        for (chunk, out) in src_u8.chunks_exact(3).zip(f64_buf.iter_mut()) {
            out[0] = chunk[0] as f64 / 255.0;
            out[1] = chunk[1] as f64 / 255.0;
            out[2] = chunk[2] as f64 / 255.0;
        }

        group.bench_with_input(
            BenchmarkId::new("f64_to_rgb8", pixel_count),
            pixel_count,
            |b, _| b.iter(|| simd::f64_to_rgb8_batch(black_box(&f64_buf), black_box(&mut dst_u8))),
        );

        // Roundtrip
        group.bench_with_input(
            BenchmarkId::new("roundtrip", pixel_count),
            pixel_count,
            |b, _| {
                b.iter(|| {
                    simd::rgb8_to_f64_batch(black_box(&src_u8), black_box(&mut f64_buf));
                    simd::f64_to_rgb8_batch(black_box(&f64_buf), black_box(&mut dst_u8));
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_matrix_single,
    bench_matrix_batch,
    bench_gamma,
    bench_srgb_decode,
    bench_srgb_encode,
    bench_lut1d,
    bench_rgb8_batch_transform,
    bench_rgb8_conversion,
);

criterion_main!(benches);
