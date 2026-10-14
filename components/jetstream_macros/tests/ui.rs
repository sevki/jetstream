//! Compile-fail tests for the versioning DSL.
//!
//! The unit tests in the crate assert the diagnostics as values. These
//! assert that a real compilation of a bad trait actually stops, with the
//! message pointing at the offending attribute rather than somewhere in
//! the generated code.

#[test]
fn versioned_service_diagnostics() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
