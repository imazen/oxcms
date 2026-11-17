set tempdir := env("HOME") / "tmp"

# The audit is isolated from cms-tests' external Argyll source requirement.
icc-audit-check:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$HOME/tmp/oxcms-icc-audit"
    export TMPDIR="$HOME/tmp/oxcms-icc-audit"
    export CARGO_TARGET_DIR="$PWD/tools/icc-audit/target"
    cargo fmt --manifest-path tools/icc-audit/Cargo.toml -- --check
    "$HOME/work/claudehints/scripts/run-heavy" --mem 3G --jobs 2 -- cargo clippy --locked --manifest-path tools/icc-audit/Cargo.toml --all-targets --all-features --no-deps -- -D warnings 2>&1 | tee "$HOME/tmp/oxcms-icc-audit/clippy.log"

icc-audit:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$HOME/tmp/oxcms-icc-audit"
    export TMPDIR="$HOME/tmp/oxcms-icc-audit"
    export CARGO_TARGET_DIR="$PWD/tools/icc-audit/target"
    "$HOME/work/claudehints/scripts/run-heavy" --mem 3G --jobs 2 -- cargo test --locked --manifest-path tools/icc-audit/Cargo.toml -- --nocapture 2>&1 | tee "$HOME/tmp/oxcms-icc-audit/default.log"

icc-audit-extended:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$HOME/tmp/oxcms-icc-audit"
    export TMPDIR="$HOME/tmp/oxcms-icc-audit"
    export CARGO_TARGET_DIR="$PWD/tools/icc-audit/target"
    "$HOME/work/claudehints/scripts/run-heavy" --mem 3G --jobs 2 -- cargo test --locked --manifest-path tools/icc-audit/Cargo.toml --features extended-range -- --nocapture 2>&1 | tee "$HOME/tmp/oxcms-icc-audit/extended.log"

# Example: just icc-audit-lcms /path/to/lcms2 (C source, not the Rust port).
icc-audit-lcms source:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "$HOME/tmp/oxcms-icc-audit" tools/icc-audit/target
    export TMPDIR="$HOME/tmp/oxcms-icc-audit"
    "$HOME/work/claudehints/scripts/run-heavy" --mem 3G --jobs 1 -- cc -O1 -I'{{source}}/include' tools/icc-audit/lcms_probe.c {{source}}/src/*.c -lm -lpthread -o tools/icc-audit/target/lcms-probe 2>&1 | tee "$HOME/tmp/oxcms-icc-audit/lcms-build.log"
    tools/icc-audit/target/lcms-probe | tee "$HOME/tmp/oxcms-icc-audit/lcms.log"
