//! The derive macros must reject invalid input with helpful errors.

#[test]
#[cfg_attr(miri, ignore = "trybuild runs cargo")]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
