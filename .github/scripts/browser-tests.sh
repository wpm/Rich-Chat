#!/usr/bin/env bash
# The browser tests: the library's and the app's code that only runs
# with a DOM (clicks and keys, the focus, the clipboard, scrolling, local
# storage), built for wasm32 and run in headless Chrome by
# wasm-bindgen-test-runner. One list for every job that runs them, as
# library-tests.sh is for the native ones.
#
#   browser-tests.sh <command>...
#
# Runs <command> once per package with the package and target flags
# after it: `browser-tests.sh cargo test`, or, for coverage (see the
# browser coverage job in ci.yml), `browser-tests.sh cargo llvm-cov
# --no-report`.
#
# Needs wasm-bindgen-test-runner on the PATH, at the wasm-bindgen
# version in Cargo.lock, and Chrome with a matching chromedriver, found
# on the PATH or named by CHROMEDRIVER.
set -euo pipefail

[ $# -gt 0 ] || { echo "usage: browser-tests.sh <command>..." >&2; exit 2; }
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
"$@" -p leptos-rich-chat --lib --target wasm32-unknown-unknown
"$@" -p rich-chat-app --target wasm32-unknown-unknown
exit 0
