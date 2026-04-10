#[test]
fn valid_attribute_passes() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/valid_attribute.rs");
}
