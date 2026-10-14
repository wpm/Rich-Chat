#!/usr/bin/env bash
# Test coverage as CI measures it, native and in the browser together,
# on a developer's machine: the runs of library-tests.sh and of
# browser-tests.sh, instrumented, and one report over both. CI runs the
# same tests in its coverage and browser coverage jobs, and Codecov adds
# their uploads up the same way.
#
#   coverage.sh [report options]    default: --html --open
#
# The options go to `cargo llvm-cov report`: `--html --open` for a page
# in the browser, nothing for a table in the terminal, `--lcov
# --output-path lcov.info` for an editor.
#
# Both runs use the nightly the browser coverage job pins, read from
# ci.yml, so that their profiles merge; rustup installs it, with the
# wasm32 target and llvm-tools, on first use. Needs cargo-llvm-cov,
# wasm-bindgen-cli at Cargo.lock's wasm-bindgen version, and Chrome; the
# chromedriver matching Chrome is fetched into target/ with npx unless
# CHROMEDRIVER names one.
set -euo pipefail
cd "$(dirname "$0")/../.."

toolchain=$(sed -n 's/^ *toolchain: \(nightly-[0-9-]*\)$/\1/p' .github/workflows/ci.yml)
[ -n "$toolchain" ] || { echo "no pinned nightly in ci.yml" >&2; exit 1; }
rustup toolchain install "$toolchain" --profile minimal \
    --target wasm32-unknown-unknown --component llvm-tools-preview >/dev/null

# wasm-bindgen-test-runner must be the version wasm-bindgen is locked to.
wanted=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
have=$(wasm-bindgen-test-runner --version 2>/dev/null | awk '{print $2}' || true)
if [ "$have" != "$wanted" ]; then
    echo "need wasm-bindgen-test-runner $wanted (have ${have:-none}):" >&2
    echo "  cargo install wasm-bindgen-cli --version $wanted --locked" >&2
    exit 1
fi

if [ -z "${CHROMEDRIVER:-}" ]; then
    chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    [ -x "$chrome" ] || chrome=$(command -v google-chrome || command -v chromium || true)
    [ -n "$chrome" ] || { echo "no Chrome found; set CHROMEDRIVER" >&2; exit 1; }
    version=$("$chrome" --version | grep -o '[0-9][0-9.]*' | head -1)
    drivers="$PWD/target/chromedriver"
    CHROMEDRIVER=$(find "$drivers" -type f -name chromedriver -path "*$version*" 2>/dev/null | head -1 || true)
    if [ -z "$CHROMEDRIVER" ]; then
        npx -y @puppeteer/browsers install "chromedriver@$version" --path "$drivers" >/dev/null
        CHROMEDRIVER=$(find "$drivers" -type f -name chromedriver -path "*$version*" | head -1)
    fi
    export CHROMEDRIVER
fi

# A target directory of its own, so that the nightly's builds do not
# displace the stable ones of `cargo test` and `cargo llvm-cov`, emptied
# of the profiles of any earlier run.
export CARGO_LLVM_COV_TARGET_DIR="$PWD/target/coverage"
mkdir -p "$CARGO_LLVM_COV_TARGET_DIR"
find "$CARGO_LLVM_COV_TARGET_DIR" -maxdepth 1 -name '*.profraw' -delete

.github/scripts/library-tests.sh cargo "+$toolchain" llvm-cov --no-report

# As the browser coverage job sets them; see the comment on it in ci.yml.
sysroot=$(rustc "+$toolchain" --print sysroot)
host=$(rustc "+$toolchain" -vV | sed -n 's/^host: //p')
AR_wasm32_unknown_unknown="$sysroot/lib/rustlib/$host/bin/llvm-ar" \
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-Zno-profiler-runtime -Clink-args=--no-gc-sections --cfg=wasm_bindgen_unstable_test_coverage" \
    .github/scripts/browser-tests.sh cargo "+$toolchain" llvm-cov --no-report

# Leaving out what codecov.yml ignores.
[ $# -gt 0 ] || set -- --html --open
cargo "+$toolchain" llvm-cov report --target wasm32-unknown-unknown \
    --ignore-filename-regex 'app/src-tauri/|app/e2e/|app/src/(main|controls|opener)\.rs' "$@"
