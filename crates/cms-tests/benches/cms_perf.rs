//! Interleaved CMS transform benchmarks using zenbench.
//!
//! Compares moxcms, skcms, lcms2, and ArgyllCMS across:
//!   - Matrix-shaper profiles (sRGB identity)
//!   - LUT-based profiles (real A2B table profiles from the wild)
//!   - Data types: u8, u16, f32
//!   - Pixel counts: 256, 4096, 65536
//!
//! Run: cargo bench -p cms-tests --bench cms_perf
//! Save baseline: cargo bench -p cms-tests --bench cms_perf -- --save-baseline=main

use std::path::PathBuf;
use zenbench::prelude::*;

// ── Helpers ─────────────────────────────────────────────────────────────

fn gen_rgb_u8(n: usize) -> Vec<u8> {
    (0..n * 3).map(|i| (i % 256) as u8).collect()
}

fn gen_rgb_u16(n: usize) -> Vec<u16> {
    (0..n * 3).map(|i| ((i * 257) % 65536) as u16).collect()
}

fn gen_rgb_f32(n: usize) -> Vec<f32> {
    (0..n * 3).map(|i| (i as f32) / (n * 3) as f32).collect()
}

fn icc_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/lilith".into());
    PathBuf::from(home).join(".cache/zenpixels-icc")
}

fn load_profile(name: &str) -> Option<Vec<u8>> {
    std::fs::read(icc_cache_dir().join(name)).ok()
}

/// Wrapper so skcms_ICCProfile can cross the Send boundary in bench closures.
/// skcms profiles from skcms_sRGB_profile() point to static data.
/// For parsed profiles, we keep the backing buffer alive in the wrapper.
struct SendProfile {
    _backing: Option<Vec<u8>>,
    profile: skcms_sys::skcms_ICCProfile,
}

// SAFETY: skcms_ICCProfile is a plain data struct with pointers into the
// backing buffer (which we co-own) or into static memory (sRGB). It has no
// thread-local state. We never mutate the profile after creation.
unsafe impl Send for SendProfile {}
unsafe impl Sync for SendProfile {}

impl SendProfile {
    fn srgb() -> Self {
        let profile = unsafe {
            std::ptr::read(skcms_sys::srgb_profile() as *const skcms_sys::skcms_ICCProfile)
        };
        Self {
            _backing: None,
            profile,
        }
    }

    fn parse(data: &[u8]) -> Option<Self> {
        // We must keep the data alive since skcms stores pointers into it
        let backing = data.to_vec();
        let profile = skcms_sys::parse_icc_profile(&backing)?;
        Some(Self {
            _backing: Some(backing),
            profile,
        })
    }

    fn get(&self) -> &skcms_sys::skcms_ICCProfile {
        &self.profile
    }
}

// ── Matrix-shaper: sRGB identity u8 ─────────────────────────────────────

fn bench_srgb_identity_u8(suite: &mut Suite) {
    for npix in [256usize, 4096, 65536] {
        suite.group(format!("sRGB_u8_{npix}px"), |g| {
            g.throughput(Throughput::Bytes((npix * 3) as u64));

            let input = gen_rgb_u8(npix);

            // moxcms
            {
                let mox_srgb = moxcms::ColorProfile::new_srgb();
                let xf = mox_srgb
                    .create_transform_8bit(
                        moxcms::Layout::Rgb,
                        &mox_srgb,
                        moxcms::Layout::Rgb,
                        moxcms::TransformOptions::default(),
                    )
                    .unwrap();
                let input = input.clone();
                g.bench("moxcms", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        xf.transform(black_box(&input), black_box(&mut out))
                            .unwrap();
                    })
                });
            }

            // skcms
            {
                let sp = SendProfile::srgb();
                let input = input.clone();
                g.bench("skcms", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        skcms_sys::transform(
                            black_box(&input),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            black_box(&mut out),
                            skcms_sys::skcms_PixelFormat::RGB_888,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            npix,
                        );
                    })
                });
            }

            // lcms2
            {
                let s = lcms2::Profile::new_srgb();
                let xf = lcms2::Transform::new(
                    &s,
                    lcms2::PixelFormat::RGB_8,
                    &s,
                    lcms2::PixelFormat::RGB_8,
                    lcms2::Intent::Perceptual,
                )
                .unwrap();
                let input = input.clone();
                g.bench("lcms2", move |b| {
                    let mut out = vec![0u8; npix * 3];
                    b.iter(|| {
                        xf.transform_pixels(black_box(&input), black_box(&mut out));
                    })
                });
            }
        });
    }
}

// ── Matrix-shaper: sRGB identity u16 ────────────────────────────────────

fn bench_srgb_identity_u16(suite: &mut Suite) {
    for npix in [256usize, 4096, 65536] {
        suite.group(format!("sRGB_u16_{npix}px"), |g| {
            g.throughput(Throughput::Bytes((npix * 6) as u64));

            let input = gen_rgb_u16(npix);

            // moxcms
            {
                let s = moxcms::ColorProfile::new_srgb();
                let xf = s
                    .create_transform_16bit(
                        moxcms::Layout::Rgb,
                        &s,
                        moxcms::Layout::Rgb,
                        moxcms::TransformOptions::default(),
                    )
                    .unwrap();
                let input = input.clone();
                g.bench("moxcms", move |b| {
                    let mut out = vec![0u16; npix * 3];
                    b.iter(|| {
                        xf.transform(black_box(&input), black_box(&mut out))
                            .unwrap();
                    })
                });
            }

            // skcms
            {
                let sp = SendProfile::srgb();
                let input = input.clone();
                g.bench("skcms", move |b| {
                    let mut out = vec![0u16; npix * 3];
                    b.iter(|| {
                        skcms_sys::transform_u16(
                            black_box(&input),
                            skcms_sys::skcms_PixelFormat::RGB_161616LE,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            black_box(&mut out),
                            skcms_sys::skcms_PixelFormat::RGB_161616LE,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            npix,
                        );
                    })
                });
            }

            // lcms2
            {
                let s = lcms2::Profile::new_srgb();
                let xf: lcms2::Transform<[u16; 3], [u16; 3]> = lcms2::Transform::new(
                    &s,
                    lcms2::PixelFormat::RGB_16,
                    &s,
                    lcms2::PixelFormat::RGB_16,
                    lcms2::Intent::Perceptual,
                )
                .unwrap();
                let input = input.clone();
                g.bench("lcms2", move |b| {
                    let src: Vec<[u16; 3]> =
                        input.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                    let mut dst = vec![[0u16; 3]; npix];
                    b.iter(|| {
                        xf.transform_pixels(black_box(&src), black_box(&mut dst));
                    })
                });
            }
        });
    }
}

// ── Matrix-shaper: sRGB identity f32 ────────────────────────────────────

fn bench_srgb_identity_f32(suite: &mut Suite) {
    for npix in [256usize, 4096, 65536] {
        suite.group(format!("sRGB_f32_{npix}px"), |g| {
            g.throughput(Throughput::Bytes((npix * 12) as u64));

            let input = gen_rgb_f32(npix);

            // moxcms
            {
                let s = moxcms::ColorProfile::new_srgb();
                let xf = s
                    .create_transform_f32(
                        moxcms::Layout::Rgb,
                        &s,
                        moxcms::Layout::Rgb,
                        moxcms::TransformOptions::default(),
                    )
                    .unwrap();
                let input = input.clone();
                g.bench("moxcms", move |b| {
                    let mut out = vec![0f32; npix * 3];
                    b.iter(|| {
                        xf.transform(black_box(&input), black_box(&mut out))
                            .unwrap();
                    })
                });
            }

            // skcms
            {
                let sp = SendProfile::srgb();
                let input = input.clone();
                g.bench("skcms", move |b| {
                    let mut out = vec![0f32; npix * 3];
                    b.iter(|| {
                        skcms_sys::transform_f32(
                            black_box(&input),
                            skcms_sys::skcms_PixelFormat::RGB_fff,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            black_box(&mut out),
                            skcms_sys::skcms_PixelFormat::RGB_fff,
                            skcms_sys::skcms_AlphaFormat::Opaque,
                            sp.get(),
                            npix,
                        );
                    })
                });
            }
        });
    }
}

// ── LUT-based profiles: u8 ──────────────────────────────────────────────

fn bench_lut_u8(suite: &mut Suite) {
    let lut_profiles: &[(&str, &str)] = &[
        ("AdobeCS4-RGB-VideoHD.icc", "VideoHD"),
        ("AdobeCS4-RGB-VideoPAL.icc", "VideoPAL"),
        ("skcms-Kodak_sRGB.icc", "KodakSRGB"),
    ];

    for &(filename, label) in lut_profiles {
        let icc_data = match load_profile(filename) {
            Some(d) => d,
            None => {
                eprintln!("SKIP: {filename} not found");
                continue;
            }
        };

        for npix in [256usize, 4096, 65536] {
            suite.group(format!("{label}_u8_{npix}px"), |g| {
                g.throughput(Throughput::Bytes((npix * 3) as u64));

                let input = gen_rgb_u8(npix);

                // moxcms
                if let Ok(src) = moxcms::ColorProfile::new_from_slice(&icc_data) {
                    let dst = moxcms::ColorProfile::new_srgb();
                    if let Ok(xf) = src.create_transform_8bit(
                        moxcms::Layout::Rgb,
                        &dst,
                        moxcms::Layout::Rgb,
                        moxcms::TransformOptions::default(),
                    ) {
                        let input = input.clone();
                        g.bench("moxcms", move |b| {
                            let mut out = vec![0u8; npix * 3];
                            b.iter(|| {
                                xf.transform(black_box(&input), black_box(&mut out))
                                    .unwrap();
                            })
                        });
                    }
                }

                // skcms
                if let Some(sp) = SendProfile::parse(&icc_data) {
                    let srgb = SendProfile::srgb();
                    let input = input.clone();
                    g.bench("skcms", move |b| {
                        let mut out = vec![0u8; npix * 3];
                        b.iter(|| {
                            skcms_sys::transform(
                                black_box(&input),
                                skcms_sys::skcms_PixelFormat::RGB_888,
                                skcms_sys::skcms_AlphaFormat::Opaque,
                                sp.get(),
                                black_box(&mut out),
                                skcms_sys::skcms_PixelFormat::RGB_888,
                                skcms_sys::skcms_AlphaFormat::Opaque,
                                srgb.get(),
                                npix,
                            );
                        })
                    });
                }

                // lcms2
                if let Ok(src) = lcms2::Profile::new_icc(&icc_data) {
                    let dst = lcms2::Profile::new_srgb();
                    if let Ok(xf) = lcms2::Transform::new(
                        &src,
                        lcms2::PixelFormat::RGB_8,
                        &dst,
                        lcms2::PixelFormat::RGB_8,
                        lcms2::Intent::Perceptual,
                    ) {
                        let input = input.clone();
                        g.bench("lcms2", move |b| {
                            let mut out = vec![0u8; npix * 3];
                            b.iter(|| {
                                xf.transform_pixels(black_box(&input), black_box(&mut out));
                            })
                        });
                    }
                }
            });
        }
    }
}

// ── LUT-based profiles: u16 ─────────────────────────────────────────────

fn bench_lut_u16(suite: &mut Suite) {
    let lut_profiles: &[(&str, &str)] = &[
        ("AdobeCS4-RGB-VideoHD.icc", "VideoHD"),
        ("skcms-Kodak_sRGB.icc", "KodakSRGB"),
    ];

    for &(filename, label) in lut_profiles {
        let icc_data = match load_profile(filename) {
            Some(d) => d,
            None => continue,
        };

        for npix in [256usize, 4096, 65536] {
            suite.group(format!("{label}_u16_{npix}px"), |g| {
                g.throughput(Throughput::Bytes((npix * 6) as u64));

                let input = gen_rgb_u16(npix);

                // moxcms
                if let Ok(src) = moxcms::ColorProfile::new_from_slice(&icc_data) {
                    let dst = moxcms::ColorProfile::new_srgb();
                    if let Ok(xf) = src.create_transform_16bit(
                        moxcms::Layout::Rgb,
                        &dst,
                        moxcms::Layout::Rgb,
                        moxcms::TransformOptions::default(),
                    ) {
                        let input = input.clone();
                        g.bench("moxcms", move |b| {
                            let mut out = vec![0u16; npix * 3];
                            b.iter(|| {
                                xf.transform(black_box(&input), black_box(&mut out))
                                    .unwrap();
                            })
                        });
                    }
                }

                // skcms
                if let Some(sp) = SendProfile::parse(&icc_data) {
                    let srgb = SendProfile::srgb();
                    let input = input.clone();
                    g.bench("skcms", move |b| {
                        let mut out = vec![0u16; npix * 3];
                        b.iter(|| {
                            skcms_sys::transform_u16(
                                black_box(&input),
                                skcms_sys::skcms_PixelFormat::RGB_161616LE,
                                skcms_sys::skcms_AlphaFormat::Opaque,
                                sp.get(),
                                black_box(&mut out),
                                skcms_sys::skcms_PixelFormat::RGB_161616LE,
                                skcms_sys::skcms_AlphaFormat::Opaque,
                                srgb.get(),
                                npix,
                            );
                        })
                    });
                }

                // lcms2
                if let Ok(src) = lcms2::Profile::new_icc(&icc_data) {
                    let dst = lcms2::Profile::new_srgb();
                    if let Ok(xf) = lcms2::Transform::<[u16; 3], [u16; 3]>::new(
                        &src,
                        lcms2::PixelFormat::RGB_16,
                        &dst,
                        lcms2::PixelFormat::RGB_16,
                        lcms2::Intent::Perceptual,
                    ) {
                        let input = input.clone();
                        g.bench("lcms2", move |b| {
                            let src_px: Vec<[u16; 3]> =
                                input.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                            let mut dst_px = vec![[0u16; 3]; npix];
                            b.iter(|| {
                                xf.transform_pixels(black_box(&src_px), black_box(&mut dst_px));
                            })
                        });
                    }
                }
            });
        }
    }
}

zenbench::main!(
    bench_srgb_identity_u8,
    bench_srgb_identity_u16,
    bench_srgb_identity_f32,
    bench_lut_u8,
    bench_lut_u16,
);
