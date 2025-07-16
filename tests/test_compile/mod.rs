#[test]
#[cfg(feature = "bits")]
#[cfg_attr(miri, ignore)]
fn test_compile() {
    let t = trybuild::TestCases::new();
    t.pass("tests/test_compile/pass_cases/*.rs");
    t.compile_fail("tests/test_compile/cases/*.rs");
}
