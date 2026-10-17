//! Code that must not compile against the crate, each case a file in
//! tests/ui with the compiler's error beside it. Not doctests: a
//! `compile_fail` doctest prints the error it expects whenever the
//! doctests' output is shown.
//!
//! Ignored by default, since trybuild builds the crate and its
//! dependencies again in a target directory of its own, and the errors
//! do not depend on the features or the instrumentation that the other
//! test runs vary. The library job in ci.yml runs it once:
//! `cargo test -p leptos-rich-chat --test ui -- --ignored`.
#![cfg(not(target_arch = "wasm32"))]

#[test]
#[ignore = "builds the crate again; run with --ignored"]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
