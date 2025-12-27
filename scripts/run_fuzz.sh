#!/bin/bash
# Run moxcms fuzzing harnesses
#
# Prerequisites:
#   cargo install cargo-fuzz
#
# Usage:
#   ./scripts/run_fuzz.sh                    # Run profile parsing fuzzer (default)
#   ./scripts/run_fuzz.sh safe_read          # Run profile parsing fuzzer
#   ./scripts/run_fuzz.sh safe_read_create   # Run transform creation fuzzer
#   ./scripts/run_fuzz.sh lut                # Run LUT handling fuzzer
#   ./scripts/run_fuzz.sh --list             # List available targets
#   ./scripts/run_fuzz.sh --corpus           # Seed corpus from test profiles

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
MOXCMS_DIR="$ROOT_DIR/external/moxcms"
CORPUS_DIR="$ROOT_DIR/testdata/profiles"

if ! command -v cargo-fuzz &> /dev/null; then
    echo "cargo-fuzz not found. Install with: cargo install cargo-fuzz"
    exit 1
fi

TARGET="${1:-safe_read}"

case "$TARGET" in
    --list)
        echo "Available fuzz targets:"
        echo "  safe_read        - Fuzz profile parsing (safe mode)"
        echo "  safe_read_create - Fuzz profile parsing + transform creation"
        echo "  lut              - Fuzz LUT handling"
        echo "  unsafe           - Fuzz with unsafe options enabled"
        exit 0
        ;;
    --corpus)
        echo "Seeding corpus from test profiles..."
        CORPUS_TARGET="$MOXCMS_DIR/fuzz/corpus/safe_read"
        mkdir -p "$CORPUS_TARGET"

        # Copy all non-fuzz ICC profiles as seed corpus
        find "$CORPUS_DIR" -name "*.icc" ! -path "*/fuzz/*" -exec cp {} "$CORPUS_TARGET/" \;

        COUNT=$(ls -1 "$CORPUS_TARGET" | wc -l)
        echo "Seeded $COUNT profiles into corpus"
        exit 0
        ;;
    safe_read|safe_read_create|lut|unsafe)
        echo "Running fuzzer: $TARGET"
        cd "$MOXCMS_DIR"
        cargo +nightly fuzz run "$TARGET" -- -max_len=1048576 -timeout=5
        ;;
    *)
        echo "Unknown target: $TARGET"
        echo "Run with --list to see available targets"
        exit 1
        ;;
esac
