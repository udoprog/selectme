#[rustversion::since(1.68)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/*_pass.rs");
    t.compile_fail("tests/ui/*_fail.rs");
}
