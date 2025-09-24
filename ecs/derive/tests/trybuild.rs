#[test]
fn derive_macros_compile() {
    let t = trybuild::TestCases::new();
    t.pass("tests/component_pass.rs");
    t.pass("tests/resource_pass.rs");
    t.pass("tests/system_pass.rs");
}
