//! Formations the compiler must reject, with the errors it reports.

#[test]
fn rejected() {
    trybuild::TestCases::new().compile_fail("tests/ui/*/*.rs");
}
