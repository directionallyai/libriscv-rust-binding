#[test]
fn syscall_registry_expands() {
    let t = trybuild::TestCases::new();
    t.pass("tests/fixtures/syscall_registry_ok.rs");
    t.compile_fail("tests/fixtures/syscall_registry_duplicate.rs");
    t.compile_fail("tests/fixtures/syscall_registry_out_of_range.rs");
}
