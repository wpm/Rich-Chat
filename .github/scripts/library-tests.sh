#!/usr/bin/env bash
# The library's test runs, one list for every job that runs them (the
# library and coverage jobs in ci.yml, and publish-crate.yml), so that
# they test the same thing: the default features, none, and together
# with the app, whose csr feature unifies into the library's test build
# and turns reactive effects on.
#
#   library-tests.sh <command>...
#
# Runs <command> once per set with the package and feature flags after
# it: `library-tests.sh cargo test`, or `library-tests.sh cargo llvm-cov
# --no-report`.
set -euo pipefail

[ $# -gt 0 ] || { echo "usage: library-tests.sh <command>..." >&2; exit 2; }
"$@" -p leptos-rich-chat
"$@" -p leptos-rich-chat --no-default-features
"$@" -p leptos-rich-chat -p rich-chat-app
