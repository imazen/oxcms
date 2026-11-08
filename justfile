# Test equipment only. Run builds/matrices through the host resource wrapper.
# Download the official archive separately; extraction verifies its pinned hash.
argyll-extract archive destination:
    python3 tools/extract_argyll_icc.py "{{archive}}" "{{destination}}"

argyll-cmyk-build sources binary:
    test ! -e "{{binary}}"
    cc -O2 -I "{{sources}}" tools/cmyk_argyll_oracle.c "{{sources}}/icc.c" -lm -o "{{binary}}"

# The Rust evaluator comes from zenpixels/cms-contract; the C oracle uses LCMS 2.20.
cmyk-matrix c_lcms rust_eval argyll output *options:
    PYTHONDONTWRITEBYTECODE=1 python3 tools/qualify_cmyk.py --profiles-root . --c-lcms "{{c_lcms}}" --rust-eval "{{rust_eval}}" --argyll "{{argyll}}" --output "{{output}}" {{options}}

cmyk-layouts matrix rust_eval c_lcms output:
    PYTHONDONTWRITEBYTECODE=1 python3 tools/qualify_cmyk_layouts.py --matrix-report "{{matrix}}" --profiles-root . --evaluator "{{rust_eval}}" --c-oracle "{{c_lcms}}" --output "{{output}}"

cmyk-native source rgb input c_lcms rust_eval argyll output:
    PYTHONDONTWRITEBYTECODE=1 python3 tools/qualify_native_cmyk.py --source "{{source}}" --rgb-profile "{{rgb}}" --inverted-input "{{input}}" --c-lcms "{{c_lcms}}" --rust-eval "{{rust_eval}}" --argyll "{{argyll}}" --output "{{output}}"
