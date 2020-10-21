#[test]
#[cfg(not(tarpaulin))]
fn test_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/test_compile/cases/*.rs");
}
