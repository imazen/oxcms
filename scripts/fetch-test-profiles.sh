#!/bin/bash
# Fetch ICC profiles for testing from various sources

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TESTDATA="$PROJECT_ROOT/testdata"

echo "=== Fetching test ICC profiles ==="

# Create directories
mkdir -p "$TESTDATA/profiles"
mkdir -p "$TESTDATA/corpus/lcms2"
mkdir -p "$TESTDATA/corpus/skcms"
mkdir -p "$TESTDATA/corpus/qcms"

# -----------------------------------------------------------------------------
# Standard profiles from color.org and others
# -----------------------------------------------------------------------------
echo "Fetching standard profiles..."

cd "$TESTDATA/profiles"

# sRGB v4 from ICC
if [ ! -f "sRGB.icc" ]; then
    curl -sL -o sRGB.icc "https://www.color.org/sRGB_v4_ICC_preference.icc"
    echo "  Downloaded sRGB.icc"
fi

# Display P3
if [ ! -f "DisplayP3.icc" ]; then
    curl -sL -o DisplayP3.icc "https://github.com/niclasko/XS/raw/master/icc/Display%20P3.icc"
    echo "  Downloaded DisplayP3.icc"
fi

# Adobe RGB
if [ ! -f "AdobeRGB1998.icc" ]; then
    curl -sL -o AdobeRGB1998.icc "https://github.com/niclasko/XS/raw/master/icc/AdobeRGB1998.icc"
    echo "  Downloaded AdobeRGB1998.icc"
fi

# Rec2020
if [ ! -f "Rec2020.icc" ]; then
    curl -sL -o Rec2020.icc "https://github.com/niclasko/XS/raw/master/icc/Rec2020.icc"
    echo "  Downloaded Rec2020.icc"
fi

# -----------------------------------------------------------------------------
# lcms2 testbed profiles
# -----------------------------------------------------------------------------
echo "Fetching lcms2 testbed profiles..."

cd "$TESTDATA/corpus/lcms2"

LCMS2_BASE="https://raw.githubusercontent.com/mm2/Little-CMS/master/testbed"

# Fetch known test profiles from lcms2
for profile in UncoatedFOGRA29.icc; do
    if [ ! -f "$profile" ]; then
        curl -sL -o "$profile" "$LCMS2_BASE/$profile" 2>/dev/null || echo "  Could not fetch $profile"
        [ -f "$profile" ] && echo "  Downloaded $profile"
    fi
done

# -----------------------------------------------------------------------------
# skcms profiles
# -----------------------------------------------------------------------------
echo "Fetching skcms profiles..."

cd "$TESTDATA/corpus/skcms"

SKCMS_BASE="https://skia.googlesource.com/skcms/+/refs/heads/main/profiles"

# skcms has profiles in subdirectories, fetch the color.org ones
for profile in sRGB2014.icc; do
    if [ ! -f "$profile" ]; then
        # Note: Google's git hosting requires special URL format for raw files
        curl -sL -o "$profile" "https://skia.googlesource.com/skcms/+/refs/heads/main/profiles/color.org/$profile?format=TEXT" 2>/dev/null
        # The response is base64 encoded
        if [ -f "$profile" ]; then
            base64 -d "$profile" > "${profile}.tmp" && mv "${profile}.tmp" "$profile"
            echo "  Downloaded $profile"
        fi
    fi
done

# -----------------------------------------------------------------------------
# qcms profiles (from Firefox)
# -----------------------------------------------------------------------------
echo "Fetching qcms profiles..."

cd "$TESTDATA/corpus/qcms"

# qcms doesn't have a separate profile corpus, uses same standard profiles
# Just symlink to the main profiles
for profile in sRGB.icc DisplayP3.icc; do
    if [ ! -f "$profile" ] && [ -f "../../profiles/$profile" ]; then
        ln -s "../../profiles/$profile" "$profile"
        echo "  Linked $profile"
    fi
done

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "=== Profile Summary ==="
echo "Standard profiles:"
ls -la "$TESTDATA/profiles/" 2>/dev/null | grep -E '\.icc$' || echo "  (none)"

echo ""
echo "lcms2 corpus:"
ls -la "$TESTDATA/corpus/lcms2/" 2>/dev/null | grep -E '\.icc$' || echo "  (none)"

echo ""
echo "skcms corpus:"
ls -la "$TESTDATA/corpus/skcms/" 2>/dev/null | grep -E '\.icc$' || echo "  (none)"

echo ""
echo "qcms corpus:"
ls -la "$TESTDATA/corpus/qcms/" 2>/dev/null | grep -E '\.icc$' || echo "  (none)"

echo ""
echo "Done!"
