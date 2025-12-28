//! ColorSync comparison test (macOS only)
//!
//! Tests whether Apple ColorSync uses colorants directly (like skcms/lcms2)
//! or applies D50 white point scaling (like original moxcms).
//!
//! This resolves the question of which behavior is "correct" when the moxcms
//! author states he targets ColorSync behavior.

#![cfg(target_os = "macos")]
#![allow(unsafe_op_in_unsafe_fn)]

use std::path::Path;
use std::ptr;

// ColorSync FFI bindings
#[link(name = "ColorSync", kind = "framework")]
unsafe extern "C" {
    fn ColorSyncProfileCreateWithURL(
        url: CFURLRef,
        options: CFDictionaryRef,
    ) -> ColorSyncProfileRef;
    fn ColorSyncTransformCreate(
        profiles: CFArrayRef,
        options: CFDictionaryRef,
    ) -> ColorSyncTransformRef;
    fn ColorSyncTransformConvert(
        transform: ColorSyncTransformRef,
        width: usize,
        height: usize,
        dst: *mut u8,
        dst_format: ColorSyncDataLayout,
        dst_bytes_per_row: usize,
        src: *const u8,
        src_format: ColorSyncDataLayout,
        src_bytes_per_row: usize,
        options: CFDictionaryRef,
    ) -> bool;
    fn ColorSyncProfileCreateWithDisplayID(display_id: u32) -> ColorSyncProfileRef;
    fn ColorSyncProfileCreateWithName(name: CFStringRef) -> ColorSyncProfileRef;
}

// ColorSync profile name constants
#[link(name = "ColorSync", kind = "framework")]
unsafe extern "C" {
    static kColorSyncSRGBProfile: CFStringRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFURLCreateWithFileSystemPath(
        allocator: CFAllocatorRef,
        file_path: CFStringRef,
        path_style: CFURLPathStyle,
        is_directory: bool,
    ) -> CFURLRef;
    fn CFStringCreateWithCString(
        allocator: CFAllocatorRef,
        c_str: *const i8,
        encoding: u32,
    ) -> CFStringRef;
    fn CFArrayCreate(
        allocator: CFAllocatorRef,
        values: *const *const std::ffi::c_void,
        num_values: isize,
        callbacks: *const std::ffi::c_void,
    ) -> CFArrayRef;
    fn CFRelease(cf: *const std::ffi::c_void);
}

type CFAllocatorRef = *const std::ffi::c_void;
type CFURLRef = *const std::ffi::c_void;
type CFStringRef = *const std::ffi::c_void;
type CFDictionaryRef = *const std::ffi::c_void;
type CFArrayRef = *const std::ffi::c_void;
type ColorSyncProfileRef = *const std::ffi::c_void;
type ColorSyncTransformRef = *const std::ffi::c_void;
type CFURLPathStyle = i32;

const kCFURLPOSIXPathStyle: CFURLPathStyle = 0;
const kCFStringEncodingUTF8: u32 = 0x08000100;

#[repr(u32)]
#[derive(Clone, Copy)]
enum ColorSyncDataLayout {
    RGB8 = 0x00000300,
}

fn testdata_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("testdata")
}

/// Transform RGB through ColorSync
unsafe fn transform_colorsync(profile_path: &str, rgb: [u8; 3]) -> Option<[u8; 3]> {
    // Create file URL
    let path_cstr = std::ffi::CString::new(profile_path).ok()?;
    let cf_path = CFStringCreateWithCString(ptr::null(), path_cstr.as_ptr(), kCFStringEncodingUTF8);
    if cf_path.is_null() {
        return None;
    }
    let url = CFURLCreateWithFileSystemPath(ptr::null(), cf_path, kCFURLPOSIXPathStyle, false);
    CFRelease(cf_path);
    if url.is_null() {
        return None;
    }

    // Create source profile from file
    let src_profile = ColorSyncProfileCreateWithURL(url, ptr::null());
    CFRelease(url);
    if src_profile.is_null() {
        return None;
    }

    // Create sRGB destination profile
    let dst_profile = ColorSyncProfileCreateWithName(kColorSyncSRGBProfile);
    if dst_profile.is_null() {
        CFRelease(src_profile);
        return None;
    }

    // Create transform
    let profiles = [src_profile, dst_profile];
    let profiles_array = CFArrayCreate(
        ptr::null(),
        profiles.as_ptr() as *const *const std::ffi::c_void,
        2,
        ptr::null(),
    );
    if profiles_array.is_null() {
        CFRelease(src_profile);
        CFRelease(dst_profile);
        return None;
    }

    let transform = ColorSyncTransformCreate(profiles_array, ptr::null());
    CFRelease(profiles_array);
    CFRelease(src_profile);
    CFRelease(dst_profile);

    if transform.is_null() {
        return None;
    }

    // Transform the color
    let mut output = [0u8; 3];
    let success = ColorSyncTransformConvert(
        transform,
        1,
        1,
        output.as_mut_ptr(),
        ColorSyncDataLayout::RGB8,
        3,
        rgb.as_ptr(),
        ColorSyncDataLayout::RGB8,
        3,
        ptr::null(),
    );

    CFRelease(transform);

    if success {
        Some(output)
    } else {
        None
    }
}

/// Transform using skcms for comparison
fn transform_skcms(profile_data: &[u8], rgb: [u8; 3]) -> Option<[u8; 3]> {
    let profile = skcms_sys::parse_icc_profile(profile_data)?;
    let srgb = skcms_sys::srgb_profile();

    let mut out = [0u8; 3];
    let success = skcms_sys::transform(
        &rgb,
        skcms_sys::skcms_PixelFormat::RGB_888,
        skcms_sys::skcms_AlphaFormat::Opaque,
        &profile,
        &mut out,
        skcms_sys::skcms_PixelFormat::RGB_888,
        skcms_sys::skcms_AlphaFormat::Opaque,
        srgb,
        1,
    );

    if success {
        Some(out)
    } else {
        None
    }
}

#[test]
fn test_colorsync_vs_skcms_sm245b() {
    let profile_path = testdata_dir().join("profiles/skcms/misc/SM245B.icc");
    if !profile_path.exists() {
        eprintln!("SKIP: SM245B.icc not found");
        return;
    }

    let profile_path_str = profile_path.to_str().unwrap();
    let profile_data = std::fs::read(&profile_path).unwrap();

    eprintln!("\n{}", "=".repeat(70));
    eprintln!("COLORSYNC vs SKCMS COMPARISON - SM245B.icc");
    eprintln!("{}\n", "=".repeat(70));

    let test_colors: &[([u8; 3], &str)] = &[
        ([255, 255, 255], "White"),
        ([128, 128, 128], "Gray 50%"),
        ([0, 0, 0], "Black"),
        ([255, 0, 0], "Red"),
        ([0, 255, 0], "Green"),
        ([0, 0, 255], "Blue"),
    ];

    eprintln!("{:<12} {:>18} {:>18} {:>8}", "Color", "ColorSync", "skcms", "Match?");
    eprintln!("{}", "-".repeat(60));

    let mut all_match = true;
    let mut colorsync_available = false;

    for (rgb, name) in test_colors {
        let colorsync_result = unsafe { transform_colorsync(profile_path_str, *rgb) };
        let skcms_result = transform_skcms(&profile_data, *rgb);

        let cs_str = match colorsync_result {
            Some([r, g, b]) => {
                colorsync_available = true;
                format!("[{:3},{:3},{:3}]", r, g, b)
            }
            None => "N/A".to_string(),
        };

        let skcms_str = match skcms_result {
            Some([r, g, b]) => format!("[{:3},{:3},{:3}]", r, g, b),
            None => "N/A".to_string(),
        };

        let matches = match (colorsync_result, skcms_result) {
            (Some(cs), Some(sk)) => {
                let max_diff = (0..3).map(|i| (cs[i] as i32 - sk[i] as i32).abs()).max().unwrap();
                if max_diff <= 1 {
                    "YES"
                } else {
                    all_match = false;
                    "NO"
                }
            }
            _ => "N/A",
        };

        eprintln!("{:<12} {:>18} {:>18} {:>8}", name, cs_str, skcms_str, matches);
    }

    eprintln!("\n{}", "=".repeat(70));

    if colorsync_available {
        if all_match {
            eprintln!("RESULT: ColorSync MATCHES skcms (uses colorants directly)");
            eprintln!("        This validates that the fix is correct.");
        } else {
            eprintln!("RESULT: ColorSync DIFFERS from skcms");
            eprintln!("        ColorSync may use D50 white point scaling.");
        }
    } else {
        eprintln!("RESULT: ColorSync not available or profile rejected");
    }
    eprintln!("{}\n", "=".repeat(70));
}
