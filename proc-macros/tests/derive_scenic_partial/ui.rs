use trybuild;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/derive_scenic_partial/fail/*.rs");
    t.pass("tests/derive_scenic_partial/pass/*.rs");
}
