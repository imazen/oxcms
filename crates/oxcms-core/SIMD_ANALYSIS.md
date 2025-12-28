# SIMD Performance Analysis

Benchmark results comparing `multiversion`-based SIMD dispatch vs scalar code.

## Summary

| Function | Speedup | Worth Further Work? |
|----------|---------|---------------------|
| LUT1D Interpolation | **1.52-1.61x** | **Yes - Best candidate** |
| Matrix batch (100k) | 1.11x | Maybe - modest gains |
| sRGB decode | 1.05-1.07x | No - `powf()` bottleneck |
| Gamma (powf) | 1.01-1.06x | No - `powf()` bottleneck |
| sRGB encode | **0.93x (slower)** | No - remove SIMD |
| Single matrix | **0.27x (slower)** | No - dispatch overhead |

## Detailed Results

### LUT1D Interpolation - Best Candidate

| Size | SIMD | Scalar | Speedup |
|------|------|--------|---------|
| 1,000 | 1.99 µs | 3.21 µs | **1.61x** |
| 10,000 | 20.1 µs | 31.3 µs | **1.56x** |
| 100,000 | 206 µs | 314 µs | **1.52x** |

**Analysis**: No `powf()` bottleneck. Pure memory access + linear interpolation.
Hand-written AVX2 intrinsics could potentially achieve 2-4x speedup.

### Matrix Batch Multiply - Moderate Gains

| Size | SIMD | Scalar | Speedup |
|------|------|--------|---------|
| 100 | 62.7 ns | 68.3 ns | 1.09x |
| 1,000 | 626 ns | 689 ns | 1.10x |
| 10,000 | 6.04 µs | 6.86 µs | 1.14x |
| 100,000 | 62.3 µs | 69.1 µs | 1.11x |

**Analysis**: ~10-14% improvement from auto-vectorization.
Explicit SIMD could improve this, but ROI is lower than LUT.

### sRGB Decode - Limited by powf()

| Size | SIMD | Scalar | Speedup |
|------|------|--------|---------|
| 1,000 | 8.13 µs | 8.52 µs | 1.05x |
| 10,000 | 80.4 µs | 86.2 µs | 1.07x |
| 100,000 | 803 µs | 862 µs | 1.07x |

**Analysis**: The `powf(2.4)` call dominates. SIMD can't vectorize `powf`.

### sRGB Encode - Scalar is Faster

| Size | SIMD | Scalar | Speedup |
|------|------|--------|---------|
| 1,000 | 8.06 µs | 7.50 µs | **0.93x** |
| 10,000 | 80.6 µs | 76.9 µs | **0.95x** |
| 100,000 | 806 µs | 747 µs | **0.93x** |

**Analysis**: Scalar is 5-8% faster! Likely due to dispatch overhead
and different branch prediction patterns.

### Single Matrix Multiply - Dispatch Overhead Dominates

| Operation | SIMD | Scalar | Speedup |
|-----------|------|--------|---------|
| Single vec3 | 7.55 ns | 2.06 ns | **0.27x** |

**Analysis**: Function call overhead (~5ns) dominates the 2ns operation.
Only use SIMD dispatch for batch operations.

## Recommendations

### High Priority - LUT Optimization

1. **Add explicit AVX2 intrinsics for LUT1D**
   - Gather 8 LUT values at once with `_mm256_i32gather_pd`
   - Vectorize linear interpolation
   - Potential: 2-4x additional speedup

2. **Add 3D LUT SIMD optimization**
   - Tetrahedral interpolation is memory-bound
   - Prefetching + gather could help significantly

### Medium Priority

3. **Consider polynomial approximation for sRGB**
   - Replace `powf(2.4)` with polynomial
   - Enables true SIMD vectorization
   - Trade accuracy for speed (configurable)

4. **Remove multiversion from sRGB encode**
   - Scalar is faster, just use direct code

### Low Priority

5. **Explicit SIMD for matrix batch**
   - Current 10% gain is acceptable
   - Only if we need more speed

## Implementation Notes

For LUT1D, the ideal AVX2 implementation:

```rust
#[cfg(target_arch = "x86_64")]
unsafe fn lut1d_avx2(input: &[f64], output: &mut [f64], lut: &[f64]) {
    use std::arch::x86_64::*;

    // Process 4 values at once with AVX2
    let lut_max = _mm256_set1_pd((lut.len() - 1) as f64);

    for (inp_chunk, out_chunk) in input.chunks_exact(4)
        .zip(output.chunks_exact_mut(4))
    {
        let x = _mm256_loadu_pd(inp_chunk.as_ptr());
        let pos = _mm256_mul_pd(x, lut_max);
        let idx = _mm256_cvttpd_epi32(pos);
        // ... gather, interpolate, store
    }
}
```

This would require `packed_simd` or direct intrinsics, which is more
complex than the current `multiversion` approach.

## Conclusion

The `multiversion` crate provides a good baseline with minimal effort.
For production performance, focus on:

1. LUT interpolation (clear 1.5x+ gain, room for 3-4x with intrinsics)
2. Avoiding SIMD dispatch for single operations
3. Polynomial approximations for transfer functions if accuracy allows
