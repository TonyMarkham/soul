#[test]
fn rejects_invalid_attribute_shapes() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/non_identifier_key.rs");
    t.compile_fail("tests/ui/duplicate_keys.rs");
    t.compile_fail("tests/ui/non_string_value.rs");
    t.compile_fail("tests/ui/empty_required_value.rs");
    t.compile_fail("tests/ui/missing_id.rs");
}
