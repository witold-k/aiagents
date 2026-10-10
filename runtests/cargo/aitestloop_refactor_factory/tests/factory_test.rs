use aitestloop_refactor_factory::Dummy;

#[test]
fn factory_refactoring() {
    // Outdated test: use DummyFactory and handle successful/failed creation.
    let dummy = Dummy::new();
    assert_eq!(dummy.value(), 1);
}
